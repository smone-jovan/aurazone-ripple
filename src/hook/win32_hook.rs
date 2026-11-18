//! Win32 low-level keyboard hook (WH_KEYBOARD_LL) with dedicated message pump thread,
//! lock-free callback state, time-critical thread priority, and silent-unhook auto-recovery.

use std::sync::atomic::{AtomicPtr, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use crossbeam_channel::Sender;
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW,
    TranslateMessage, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL,
    WM_APP, WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use super::vk_map::{map_vk_to_zone, DebounceFilter, KeyEvent};

/// Custom thread message posted by the poller watchdog when a silent unhook is detected.
pub const WM_APP_REHOOK: u32 = WM_APP + 0x42;

struct HookState {
    sender: Sender<KeyEvent>,
    debouncer: Arc<DebounceFilter>,
    epoch: Instant,
}

/// Lock-free atomic pointer to `HookState` so `low_level_keyboard_proc` never contends on a lock.
static HOOK_STATE_PTR: AtomicPtr<HookState> = AtomicPtr::new(std::ptr::null_mut());

/// Monotonic timestamp (ms since HookState.epoch) of the last event received by WH_KEYBOARD_LL.
static LAST_HOOK_CALLBACK_MS: AtomicU64 = AtomicU64::new(0);

/// Active hook worker thread ID for posting `WM_APP_REHOOK` from the poller watchdog.
static HOOK_THREAD_ID: AtomicU32 = AtomicU32::new(0);

/// Returns the last monotonic timestamp (ms) when `WH_KEYBOARD_LL` processed any key event.
#[inline]
pub fn last_hook_callback_ms() -> u64 {
    LAST_HOOK_CALLBACK_MS.load(Ordering::Relaxed)
}

/// Requests the `WH_KEYBOARD_LL` worker thread to cleanly unhook and re-install the hook.
/// Called by the poller watchdog when physical keypresses are detected while the hook is silent.
#[inline]
pub fn request_auto_rehook() {
    let tid = HOOK_THREAD_ID.load(Ordering::Acquire);
    if tid != 0 {
        unsafe {
            let _ = PostThreadMessageW(tid, WM_APP_REHOOK, 0, 0);
        }
    }
}

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
        let state_ptr = HOOK_STATE_PTR.load(Ordering::Acquire);
        if !state_ptr.is_null() {
            let state = &*state_ptr;
            let now = Instant::now();
            let now_ms = now.saturating_duration_since(state.epoch).as_millis() as u64 + 1;
            LAST_HOOK_CALLBACK_MS.store(now_ms, Ordering::Relaxed);

            let msg = wparam as u32;
            let kbd = &*(lparam as *const KBDLLHOOKSTRUCT);
            let vk_code = kbd.vkCode;

            if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
                if state.debouncer.should_process(vk_code, now) {
                    let zone = map_vk_to_zone(vk_code);
                    let _ = state.sender.try_send(KeyEvent {
                        zone,
                        vk_code,
                        timestamp: now,
                    });
                } else {
                    // Refresh hold lease while OS key-repeat events arrive
                    state.debouncer.refresh_hold_lease(vk_code, now);
                }
            } else if msg == WM_KEYUP || msg == WM_SYSKEYUP {
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
        // Register static lock-free hook state
        let boxed_state = Box::new(HookState {
            sender,
            debouncer,
            epoch: Instant::now(),
        });
        let raw_state = Box::into_raw(boxed_state);
        let old_ptr = HOOK_STATE_PTR.swap(raw_state, Ordering::AcqRel);
        if !old_ptr.is_null() {
            unsafe {
                drop(Box::from_raw(old_ptr));
            }
        }
        LAST_HOOK_CALLBACK_MS.store(1, Ordering::Relaxed);

        let (init_tx, init_rx) = crossbeam_channel::bounded(1);

        let join_handle = std::thread::Builder::new()
            .name("WH_KEYBOARD_LL_Worker".into())
            .spawn(move || {
                // Elevate hook thread to THREAD_PRIORITY_TIME_CRITICAL & disable EcoQoS
                // to prevent LowLevelHooksTimeout silent unhooking under 100% CPU load.
                crate::qos::boost_input_thread(true);

                #[cfg(windows)]
                let orig_hdesk = unsafe {
                    use windows_sys::Win32::System::StationsAndDesktops::GetThreadDesktop;
                    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
                    GetThreadDesktop(GetCurrentThreadId())
                };
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
                let h_instance = unsafe {
                    windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(std::ptr::null())
                };
                let mut hook: HHOOK = unsafe {
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

                HOOK_THREAD_ID.store(thread_id, Ordering::Release);

                // Force creation of message queue by peeking and notify starter
                let mut msg: MSG = unsafe { std::mem::zeroed() };
                unsafe {
                    use windows_sys::Win32::UI::WindowsAndMessaging::{PeekMessageW, PM_NOREMOVE};
                    PeekMessageW(&mut msg, 0 as _, 0, 0, PM_NOREMOVE);
                }
                let _ = init_tx.send(Ok(thread_id));

                // Pump Win32 message queue and handle live watchdog re-hook messages
                while unsafe { GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) } > 0 {
                    if msg.message == WM_APP_REHOOK {
                        unsafe {
                            if !hook.is_null() {
                                UnhookWindowsHookEx(hook);
                            }
                            let new_hook = SetWindowsHookExW(
                                WH_KEYBOARD_LL,
                                Some(low_level_keyboard_proc),
                                h_instance,
                                0,
                            );
                            if !new_hook.is_null() {
                                hook = new_hook;
                            }
                        }
                        continue;
                    }
                    unsafe {
                        TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }

                HOOK_THREAD_ID.store(0, Ordering::Release);

                // Clean up hook on exit
                unsafe {
                    if !hook.is_null() {
                        UnhookWindowsHookEx(hook);
                    }
                }

                #[cfg(windows)]
                if hook_hdesk != 0 as _ {
                    unsafe {
                        use windows_sys::Win32::System::StationsAndDesktops::{
                            CloseDesktop, SetThreadDesktop,
                        };
                        if orig_hdesk != 0 as _ {
                            SetThreadDesktop(orig_hdesk);
                        }
                        CloseDesktop(hook_hdesk);
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
        HOOK_THREAD_ID.store(0, Ordering::Release);
        if let Some(handle) = self.join_handle.take() {
            unsafe {
                PostThreadMessageW(self.thread_id, WM_QUIT, 0, 0);
            }
            let _ = handle.join();
        }
        let old_ptr = HOOK_STATE_PTR.swap(std::ptr::null_mut(), Ordering::AcqRel);
        if !old_ptr.is_null() {
            unsafe {
                drop(Box::from_raw(old_ptr));
            }
        }
    }
}

impl Drop for KeyboardHookHandle {
    fn drop(&mut self) {
        self.stop();
    }
}
