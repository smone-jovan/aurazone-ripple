//! Win32 Raw Input (`WM_INPUT` with `RIDEV_INPUTSINK`) capture layer.
//!
//! Provides an interrupt-driven, lock-free keyboard input channel alongside
//! `WH_KEYBOARD_LL` and the 200 Hz poller. When running with High-Integrity
//! elevation (Task Scheduler `HighestAvailable`), Raw Input receives keyboard
//! events reliably even when DirectX/Vulkan elevated anti-cheat games (e.g. ZZZ)
//! hold foreground focus.

use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::Arc;
use std::time::Instant;
use crossbeam_channel::Sender;

#[cfg(windows)]
use windows_sys::Win32::Foundation::{GetLastError, HWND, LPARAM, LRESULT, WPARAM};
#[cfg(windows)]
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(windows)]
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
#[cfg(windows)]
use windows_sys::Win32::UI::Input::{
    GetRawInputData, RegisterRawInputDevices, HRAWINPUT, RAWINPUT, RAWINPUTDEVICE,
    RAWINPUTHEADER, RIDEV_INPUTSINK, RID_INPUT, RIM_TYPEKEYBOARD,
};
#[cfg(windows)]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    PeekMessageW, PostThreadMessageW, RegisterClassW, TranslateMessage, HWND_MESSAGE, MSG,
    PM_NOREMOVE, WM_INPUT, WM_QUIT, WNDCLASSW,
};

use super::vk_map::{map_vk_to_zone, DebounceFilter, KeyEvent};

struct RawInputState {
    sender: Sender<KeyEvent>,
    debouncer: Arc<DebounceFilter>,
}

static RAW_STATE_PTR: AtomicPtr<RawInputState> = AtomicPtr::new(std::ptr::null_mut());
static RAW_INPUT_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Returns `true` when the Raw Input sink thread is currently registered and active.
#[inline]
pub fn is_raw_input_active() -> bool {
    RAW_INPUT_ACTIVE.load(Ordering::Acquire)
}

/// Normalizes a Win32 `RAWKEYBOARD` virtual keycode into distinct left/right modifier VKs
/// and filters synthetic `0xFF` overrun/prefix scancodes.
#[inline]
pub fn normalize_raw_vk(vkey: u16, make_code: u16, flags: u16) -> Option<u32> {
    if vkey == 0 || vkey == 0xFF {
        return None;
    }
    const RI_KEY_E0: u16 = 0x0002;
    let is_e0 = (flags & RI_KEY_E0) != 0;

    let mapped = match vkey {
        0x10 => {
            if make_code == 0x36 {
                0xA1 // VK_RSHIFT
            } else {
                0xA0 // VK_LSHIFT
            }
        }
        0x11 => {
            if is_e0 {
                0xA3 // VK_RCONTROL
            } else {
                0xA2 // VK_LCONTROL
            }
        }
        0x12 => {
            if is_e0 {
                0xA5 // VK_RMENU
            } else {
                0xA4 // VK_LMENU
            }
        }
        other => other as u32,
    };
    Some(mapped)
}

#[cfg(windows)]
#[repr(C, align(8))]
struct AlignedRawBuffer([u8; 64]);

#[cfg(windows)]
unsafe extern "system" fn raw_input_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_INPUT {
        let mut buf = AlignedRawBuffer([0u8; 64]);
        let mut size = std::mem::size_of::<AlignedRawBuffer>() as u32;
        let header_size = std::mem::size_of::<RAWINPUTHEADER>() as u32;

        let copied = GetRawInputData(
            lparam as HRAWINPUT,
            RID_INPUT,
            buf.0.as_mut_ptr() as *mut _,
            &mut size,
            header_size,
        );

        if copied != u32::MAX && copied >= header_size {
            let raw = &*(buf.0.as_ptr() as *const RAWINPUT);
            if raw.header.dwType == RIM_TYPEKEYBOARD {
                let kbd = raw.data.keyboard;
                if let Some(vk) = normalize_raw_vk(kbd.VKey, kbd.MakeCode, kbd.Flags) {
                    let state_ptr = RAW_STATE_PTR.load(Ordering::Acquire);
                    if !state_ptr.is_null() {
                        let state = &*state_ptr;
                        let now = Instant::now();
                        const RI_KEY_BREAK: u16 = 0x0001;
                        let is_break = (kbd.Flags & RI_KEY_BREAK) != 0;

                        if !is_break {
                            if state.debouncer.should_process(vk, now) {
                                let zone = map_vk_to_zone(vk);
                                let _ = state.sender.try_send(KeyEvent {
                                    zone,
                                    vk_code: vk,
                                    timestamp: now,
                                });
                            } else {
                                state.debouncer.refresh_hold_lease(vk, now);
                            }
                        } else {
                            state.debouncer.notify_key_up(vk);
                        }
                    }
                }
            }
        }
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}

pub struct RawInputHandle {
    thread_id: u32,
    join_handle: Option<std::thread::JoinHandle<()>>,
}

impl RawInputHandle {
    /// Starts the dedicated Raw Input message-only window thread with `RIDEV_INPUTSINK`.
    pub fn start(
        sender: Sender<KeyEvent>,
        debouncer: Arc<DebounceFilter>,
    ) -> Result<Self, crate::hook::HookError> {
        #[cfg(not(windows))]
        {
            let _ = (sender, debouncer);
            return Err(crate::hook::HookError::InitTimeout);
        }

        #[cfg(windows)]
        {
            let boxed_state = Box::new(RawInputState { sender, debouncer });
            let raw_state = Box::into_raw(boxed_state);
            let old_ptr = RAW_STATE_PTR.swap(raw_state, Ordering::AcqRel);
            if !old_ptr.is_null() {
                unsafe {
                    drop(Box::from_raw(old_ptr));
                }
            }

            let (init_tx, init_rx) = crossbeam_channel::bounded(1);

            let join_handle = std::thread::Builder::new()
                .name("RawInput_Worker".into())
                .spawn(move || {
                    crate::qos::boost_input_thread(true);

                    unsafe {
                        use windows_sys::Win32::System::StationsAndDesktops::{
                            CloseDesktop, GetThreadDesktop, OpenDesktopW, SetThreadDesktop,
                        };

                        let orig_hdesk = GetThreadDesktop(GetCurrentThreadId());
                        let mut raw_hdesk = 0 as windows_sys::Win32::Foundation::HANDLE;
                        let default_name: Vec<u16> = "Default\0".encode_utf16().collect();
                        let hdesk = OpenDesktopW(default_name.as_ptr(), 0, 0, 0x1FF);
                        if hdesk != 0 as _ {
                            if SetThreadDesktop(hdesk) != 0 {
                                raw_hdesk = hdesk;
                            } else {
                                CloseDesktop(hdesk);
                            }
                        }

                        let restore_desktop = |orig: windows_sys::Win32::Foundation::HANDLE,
                                               attached: windows_sys::Win32::Foundation::HANDLE| {
                            if attached != 0 as _ {
                                if orig != 0 as _ {
                                    SetThreadDesktop(orig);
                                }
                                CloseDesktop(attached);
                            }
                        };

                        let thread_id = GetCurrentThreadId();
                        let h_instance = GetModuleHandleW(std::ptr::null());
                        let class_name: Vec<u16> =
                            "AuraZone_RawInputClass\0".encode_utf16().collect();

                        let wc = WNDCLASSW {
                            style: 0,
                            lpfnWndProc: Some(raw_input_wnd_proc),
                            cbClsExtra: 0,
                            cbWndExtra: 0,
                            hInstance: h_instance,
                            hIcon: std::ptr::null_mut(),
                            hCursor: std::ptr::null_mut(),
                            hbrBackground: std::ptr::null_mut(),
                            lpszMenuName: std::ptr::null(),
                            lpszClassName: class_name.as_ptr(),
                        };

                        let _ = RegisterClassW(&wc);

                        let hwnd = CreateWindowExW(
                            0,
                            class_name.as_ptr(),
                            class_name.as_ptr(),
                            0,
                            0,
                            0,
                            0,
                            0,
                            HWND_MESSAGE,
                            std::ptr::null_mut(),
                            h_instance,
                            std::ptr::null(),
                        );

                        if hwnd.is_null() {
                            let err = GetLastError();
                            restore_desktop(orig_hdesk, raw_hdesk);
                            let _ = init_tx.send(Err(err));
                            return;
                        }

                        let rid = RAWINPUTDEVICE {
                            usUsagePage: 0x01, // Generic Desktop Controls
                            usUsage: 0x06,     // Keyboard
                            dwFlags: RIDEV_INPUTSINK,
                            hwndTarget: hwnd,
                        };

                        if RegisterRawInputDevices(
                            &rid,
                            1,
                            std::mem::size_of::<RAWINPUTDEVICE>() as u32,
                        ) == 0
                        {
                            let err = GetLastError();
                            DestroyWindow(hwnd);
                            restore_desktop(orig_hdesk, raw_hdesk);
                            let _ = init_tx.send(Err(err));
                            return;
                        }

                        let mut msg: MSG = std::mem::zeroed();
                        PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_NOREMOVE);

                        RAW_INPUT_ACTIVE.store(true, Ordering::Release);
                        let _ = init_tx.send(Ok(thread_id));

                        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                            TranslateMessage(&msg);
                            DispatchMessageW(&msg);
                        }

                        RAW_INPUT_ACTIVE.store(false, Ordering::Release);
                        DestroyWindow(hwnd);
                        restore_desktop(orig_hdesk, raw_hdesk);
                    }
                })
                .map_err(crate::hook::HookError::ThreadSpawnFailed)?;

            match init_rx.recv_timeout(std::time::Duration::from_secs(2)) {
                Ok(Ok(thread_id)) => Ok(Self {
                    thread_id,
                    join_handle: Some(join_handle),
                }),
                Ok(Err(err)) => {
                    let _ = join_handle.join();
                    let old_ptr = RAW_STATE_PTR.swap(std::ptr::null_mut(), Ordering::AcqRel);
                    if !old_ptr.is_null() {
                        unsafe {
                            drop(Box::from_raw(old_ptr));
                        }
                    }
                    Err(crate::hook::HookError::SetHookFailed(err))
                }
                Err(_) => Err(crate::hook::HookError::InitTimeout),
            }
        }
    }

    pub fn stop(&mut self) {
        RAW_INPUT_ACTIVE.store(false, Ordering::Release);
        if let Some(handle) = self.join_handle.take() {
            #[cfg(windows)]
            unsafe {
                PostThreadMessageW(self.thread_id, WM_QUIT, 0, 0);
            }
            let _ = handle.join();
        }
        let old_ptr = RAW_STATE_PTR.swap(std::ptr::null_mut(), Ordering::AcqRel);
        if !old_ptr.is_null() {
            unsafe {
                drop(Box::from_raw(old_ptr));
            }
        }
    }
}

impl Drop for RawInputHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_raw_vk() {
        assert_eq!(normalize_raw_vk(0xFF, 0, 0), None);
        assert_eq!(normalize_raw_vk(0, 0, 0), None);

        // Shift differentiation by scan code (0x2A = LShift, 0x36 = RShift)
        assert_eq!(normalize_raw_vk(0x10, 0x2A, 0), Some(0xA0));
        assert_eq!(normalize_raw_vk(0x10, 0x36, 0), Some(0xA1));

        // Control & Alt differentiation by RI_KEY_E0 (0x02)
        assert_eq!(normalize_raw_vk(0x11, 0x1D, 0), Some(0xA2));
        assert_eq!(normalize_raw_vk(0x11, 0x1D, 0x02), Some(0xA3));
        assert_eq!(normalize_raw_vk(0x12, 0x38, 0), Some(0xA4));
        assert_eq!(normalize_raw_vk(0x12, 0x38, 0x02), Some(0xA5));

        // Standard keys preserved
        assert_eq!(normalize_raw_vk(0x57, 0x11, 0), Some(0x57)); // 'W'
        assert_eq!(normalize_raw_vk(0x20, 0x39, 0), Some(0x20)); // Space
    }

    #[cfg(windows)]
    #[test]
    fn test_raw_input_start_and_stop_lifecycle() {
        let (tx, _rx) = crossbeam_channel::unbounded();
        let debouncer = Arc::new(DebounceFilter::default());
        let mut handle = RawInputHandle::start(tx, debouncer).expect("RawInputHandle must start");
        assert!(is_raw_input_active());
        handle.stop();
        assert!(!is_raw_input_active());
    }
}
