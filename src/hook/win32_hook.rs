//! Win32 low-level keyboard hook (WH_KEYBOARD_LL) with dedicated message pump thread.

use std::sync::Arc;
use std::time::Instant;
use crossbeam_channel::Sender;
use parking_lot::RwLock;
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW,
    TranslateMessage, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL,
    WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use super::vk_map::{map_vk_to_zone, DebounceFilter, KeyEvent};

struct HookState {
    sender: Sender<KeyEvent>,
    debouncer: Arc<DebounceFilter>,
}

static HOOK_STATE: RwLock<Option<HookState>> = RwLock::new(None);

pub struct KeyboardHookHandle {
    thread_id: u32,
    join_handle: Option<std::thread::JoinHandle<()>>,
}

unsafe extern "system" fn low_level_keyboard_proc(
    ncode: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if ncode >= 0 && lparam != 0 {
        let msg = wparam as u32;
        if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
            let kbd = &*(lparam as *const KBDLLHOOKSTRUCT);
            let vk_code = kbd.vkCode;

            if let Some(state) = HOOK_STATE.read().as_ref() {
                let now = Instant::now();
                if state.debouncer.should_process(vk_code, now) {
                    let zone = map_vk_to_zone(vk_code);
                    let _ = state.sender.try_send(KeyEvent {
                        zone,
                        vk_code,
                        timestamp: now,
                    });
                }
            }
        } else if msg == WM_KEYUP || msg == WM_SYSKEYUP {
            let kbd = &*(lparam as *const KBDLLHOOKSTRUCT);
            let vk_code = kbd.vkCode;

            if let Some(state) = HOOK_STATE.read().as_ref() {
                state.debouncer.notify_key_up(vk_code);
            }
        }
    }
    CallNextHookEx(std::ptr::null_mut(), ncode, wparam, lparam)
}

impl KeyboardHookHandle {
    pub fn start(
        sender: Sender<KeyEvent>,
        debouncer: Arc<DebounceFilter>,
    ) -> Result<Self, crate::hook::HookError> {
        // Register static hook state
        *HOOK_STATE.write() = Some(HookState { sender, debouncer });

        let (init_tx, init_rx) = crossbeam_channel::bounded(1);

        let join_handle = std::thread::Builder::new()
            .name("WH_KEYBOARD_LL_Worker".into())
            .spawn(move || {
                #[cfg(windows)]
                let mut hook_hdesk = 0 as windows_sys::Win32::Foundation::HANDLE;
                #[cfg(windows)]
                unsafe {
                    use windows_sys::Win32::System::StationsAndDesktops::{
                        CloseDesktop, OpenDesktopW, SetThreadDesktop,
                    };
                    let default_name: Vec<u16> = "Default\0".encode_utf16().collect();
                    let hdesk = OpenDesktopW(default_name.as_ptr(), 0, 0, 0x1FF);
                    if hdesk != 0 as _ {
                        if SetThreadDesktop(hdesk) != 0 {
                            hook_hdesk = hdesk;
                        } else {
                            CloseDesktop(hdesk);
                        }
                    }
                }

                let thread_id = unsafe { GetCurrentThreadId() };
                let h_instance = unsafe { windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(std::ptr::null()) };
                let hook: HHOOK = unsafe {
                    SetWindowsHookExW(
                        WH_KEYBOARD_LL,
                        Some(low_level_keyboard_proc),
                        h_instance,
                        0,
                    )
                };

                if hook.is_null() {
                    let err = unsafe { windows_sys::Win32::Foundation::GetLastError() };
                    let _ = init_tx.send(Err(err));
                    return;
                }

                // Force creation of message queue by peeking and notify starter
                let mut msg: MSG = unsafe { std::mem::zeroed() };
                unsafe {
                    use windows_sys::Win32::UI::WindowsAndMessaging::{PeekMessageW, PM_NOREMOVE};
                    PeekMessageW(&mut msg, 0 as _, 0, 0, PM_NOREMOVE);
                }
                let _ = init_tx.send(Ok(thread_id));

                // Pump Win32 message queue
                while unsafe { GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) } > 0 {
                    unsafe {
                        TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }

                // Clean up hook on exit
                unsafe {
                    UnhookWindowsHookEx(hook);
                }

                #[cfg(windows)]
                if hook_hdesk != 0 as _ {
                    unsafe {
                        windows_sys::Win32::System::StationsAndDesktops::CloseDesktop(hook_hdesk);
                    }
                }
            })
            .map_err(crate::hook::HookError::ThreadSpawnFailed)?;

        let thread_id = init_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .map_err(|_| crate::hook::HookError::InitTimeout)?
            .map_err(crate::hook::HookError::SetHookFailed)?;

        Ok(Self {
            thread_id,
            join_handle: Some(join_handle),
        })
    }

    pub fn stop(&mut self) {
        if let Some(handle) = self.join_handle.take() {
            unsafe {
                PostThreadMessageW(self.thread_id, WM_QUIT, 0, 0);
            }
            let _ = handle.join();
        }
        *HOOK_STATE.write() = None;
    }
}

impl Drop for KeyboardHookHandle {
    fn drop(&mut self) {
        self.stop();
    }
}
