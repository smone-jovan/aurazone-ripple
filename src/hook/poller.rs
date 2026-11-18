//! High-frequency 200 Hz GetAsyncKeyState poller with 1ms multimedia timer precision,
//! LSB (0x0001) fast-tap capture, stuck-bit release reconciliation, and silent-unhook watchdog.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use crossbeam_channel::Sender;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

use super::vk_map::{map_vk_to_zone, DebounceFilter, KeyEvent};
use super::win32_hook::{last_hook_callback_ms, request_auto_rehook};

/// Curated virtual keycodes covering standard keyboard layout + ROG M1-M4 keys.
pub const POLLED_KEYS: &[u32] = &[
    // Function keys (F1-F12)
    0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A, 0x7B,
    // Number row ('0'-'9', `~, -, =)
    0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39,
    0xC0, 0xBD, 0xBB,
    // Alphabet ('A'-'Z')
    0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x4B, 0x4C, 0x4D,
    0x4E, 0x4F, 0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5A,
    // Control & Navigation
    0x1B, 0x09, 0x14, 0x20, 0x0D, 0x08,
    0x10, 0x11, 0x12, // Generic VK_SHIFT, VK_CONTROL, VK_MENU for modifier release sync
    0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0x5B, 0x5C,
    0x25, 0x26, 0x27, 0x28, // Arrows (Left, Up, Right, Down)
    0x2D, 0x2E, 0x24, 0x23, 0x21, 0x22, // Ins, Del, Home, End, PgUp, PgDn
    // Punctuation ([, ], \, ;, ', ,, ., /)
    0xDB, 0xDD, 0xDC, 0xBA, 0xDE, 0xBC, 0xBE, 0xBF,
    // Numpad (0-9, *, +, -, ., /)
    0x60, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,
    0x6A, 0x6B, 0x6D, 0x6E, 0x6F,
    // ROG Audio Hotkeys (M1-M3) & Dedicated Rightmost Media Keys (Play, Stop, Prev, Next)
    0xAD, 0xAE, 0xAF, 0xB0, 0xB1, 0xB2, 0xB3,
];

/// Watchdog threshold: if the poller captures >= 3 physical keystrokes while WH_KEYBOARD_LL
/// has not fired for over 2000 ms, request an automatic re-hook.
const WATCHDOG_SILENCE_THRESHOLD_MS: u64 = 2000;
const REHOOK_COOLDOWN: Duration = Duration::from_secs(5);

pub struct PollerHandle {
    running: Arc<AtomicBool>,
    join_handle: Option<std::thread::JoinHandle<()>>,
}

impl PollerHandle {
    pub fn start(
        sender: Sender<KeyEvent>,
        debouncer: Arc<DebounceFilter>,
    ) -> Result<Self, crate::hook::HookError> {
        let running = Arc::new(AtomicBool::new(true));
        let thread_running = running.clone();

        let join_handle = std::thread::Builder::new()
            .name("GetAsyncKeyStatePoller".into())
            .spawn(move || {
                // Elevate poller thread to THREAD_PRIORITY_HIGHEST, disable EcoQoS,
                // and engage 1ms multimedia timer resolution for accurate 5ms (200 Hz) sleeps.
                crate::qos::boost_input_thread(false);
                let _timer_guard = crate::qos::HighResTimerGuard::new(1);

                let mut last_states = [false; 256];
                let mut missed_hook_strokes: u32 = 0;
                let mut last_rehook_attempt = Instant::now();
                let mut last_seen_hook_ms: u64 = last_hook_callback_ms();
                let mut hook_silence_start = Instant::now();

                #[cfg(windows)]
                let orig_hdesk = unsafe {
                    use windows_sys::Win32::System::StationsAndDesktops::GetThreadDesktop;
                    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
                    GetThreadDesktop(GetCurrentThreadId())
                };
                #[cfg(windows)]
                let mut attached_hdesk = 0 as windows_sys::Win32::Foundation::HANDLE;

                while thread_running.load(Ordering::Relaxed) {
                    #[cfg(windows)]
                    if attached_hdesk == 0 as _ {
                        unsafe {
                            use windows_sys::Win32::System::StationsAndDesktops::{
                                CloseDesktop, OpenDesktopW, SetThreadDesktop,
                            };
                            let default_name: Vec<u16> = "Default\0".encode_utf16().collect();
                            let hdesk = OpenDesktopW(default_name.as_ptr(), 0, 0, 0x1FF);
                            if hdesk != 0 as _ {
                                if SetThreadDesktop(hdesk) != 0 {
                                    attached_hdesk = hdesk;
                                } else {
                                    CloseDesktop(hdesk);
                                }
                            }
                        }
                    }

                    let now = Instant::now();
                    let current_hook_ms = last_hook_callback_ms();
                    if current_hook_ms != last_seen_hook_ms {
                        last_seen_hook_ms = current_hook_ms;
                        hook_silence_start = now;
                        missed_hook_strokes = 0;
                    }

                    for &vk in POLLED_KEYS {
                        let idx = vk as usize;
                        let state = unsafe { GetAsyncKeyState(vk as i32) } as u16;
                        let is_down = (state & 0x8000) != 0;
                        let was_tapped = (state & 0x0001) != 0;
                        let was_down = if idx < last_states.len() {
                            last_states[idx]
                        } else {
                            false
                        };

                        if is_down && !was_down {
                            // Standard key-down edge
                            if debouncer.should_process(vk, now) {
                                let zone = map_vk_to_zone(vk);
                                let _ = sender.try_send(KeyEvent {
                                    zone,
                                    vk_code: vk,
                                    timestamp: now,
                                });
                                missed_hook_strokes = missed_hook_strokes.saturating_add(1);
                            } else {
                                debouncer.refresh_hold_lease(vk, now);
                            }
                        } else if is_down && was_down {
                            // Continuously held down: refresh lease to maintain auto-repeat suppression
                            debouncer.refresh_hold_lease(vk, now);
                        } else if !is_down && !was_down && was_tapped {
                            // Fast tap (<5-10ms counter-strafing) pressed and released between ticks
                            if debouncer.should_process(vk, now) {
                                let zone = map_vk_to_zone(vk);
                                let _ = sender.try_send(KeyEvent {
                                    zone,
                                    vk_code: vk,
                                    timestamp: now,
                                });
                                missed_hook_strokes = missed_hook_strokes.saturating_add(1);
                            }
                            // Immediately clear down_bitmap since the key is already physically up
                            debouncer.notify_key_up(vk);
                        } else if !is_down {
                            // Unconditional physical release reconciliation:
                            // Clears down_bitmap even if WH_KEYBOARD_LL set the bit and subsequently
                            // dropped WM_KEYUP due to Alt-Tab, UIPI elevation, or hook timeout.
                            if was_down || debouncer.is_key_down(vk) {
                                debouncer.notify_key_up(vk);
                            }
                        }

                        if idx < last_states.len() {
                            last_states[idx] = is_down;
                        }
                    }

                    // Watchdog check: trigger live re-hook if WH_KEYBOARD_LL silently died
                    if missed_hook_strokes >= 3
                        && hook_silence_start.elapsed().as_millis() as u64
                            >= WATCHDOG_SILENCE_THRESHOLD_MS
                        && last_rehook_attempt.elapsed() >= REHOOK_COOLDOWN
                    {
                        request_auto_rehook();
                        last_rehook_attempt = now;
                        missed_hook_strokes = 0;
                    }

                    // 200 Hz scan rate (5 ms) stabilized by timeBeginPeriod(1)
                    std::thread::sleep(Duration::from_millis(5));
                }

                #[cfg(windows)]
                if attached_hdesk != 0 as _ {
                    unsafe {
                        use windows_sys::Win32::System::StationsAndDesktops::{
                            CloseDesktop, SetThreadDesktop,
                        };
                        if orig_hdesk != 0 as _ {
                            SetThreadDesktop(orig_hdesk);
                        }
                        CloseDesktop(attached_hdesk);
                    }
                }
            })
            .map_err(crate::hook::HookError::ThreadSpawnFailed)?;

        Ok(Self {
            running,
            join_handle: Some(join_handle),
        })
    }

    pub fn stop(&mut self) {
        if let Some(handle) = self.join_handle.take() {
            self.running.store(false, Ordering::Release);
            let _ = handle.join();
        }
    }
}

impl Drop for PollerHandle {
    fn drop(&mut self) {
        self.stop();
    }
}
