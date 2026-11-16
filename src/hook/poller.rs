//! Asynchronous GetAsyncKeyState poller for UIPI elevated window keystroke capture fallback.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use crossbeam_channel::Sender;
use parking_lot::Mutex;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

use super::vk_map::{map_vk_to_zone, DebounceFilter, KeyEvent};

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

pub struct PollerHandle {
    running: Arc<AtomicBool>,
    join_handle: Option<std::thread::JoinHandle<()>>,
}

impl PollerHandle {
    pub fn start(
        sender: Sender<KeyEvent>,
        debouncer: Arc<Mutex<DebounceFilter>>,
    ) -> Result<Self, crate::hook::HookError> {
        let running = Arc::new(AtomicBool::new(true));
        let thread_running = running.clone();

        let join_handle = std::thread::Builder::new()
            .name("GetAsyncKeyStatePoller".into())
            .spawn(move || {
                let mut last_states = [false; 256];
                let mut is_attached = false;
                let mut poll_cycle: u32 = 0;

                while thread_running.load(Ordering::Relaxed) {
                    poll_cycle = poll_cycle.wrapping_add(1);
                    if !is_attached || (poll_cycle % 100 == 0) {
                        #[cfg(windows)]
                        unsafe {
                            use windows_sys::Win32::System::StationsAndDesktops::{OpenDesktopW, SetThreadDesktop};
                            let default_name: Vec<u16> = "Default\0".encode_utf16().collect();
                            let hdesk = OpenDesktopW(default_name.as_ptr(), 0, 0, 0x1FF);
                            if hdesk != 0 as _ {
                                if SetThreadDesktop(hdesk) != 0 {
                                    is_attached = true;
                                }
                            }
                        }
                    }

                    for &vk in POLLED_KEYS {
                        let state = unsafe { GetAsyncKeyState(vk as i32) };
                        let is_down = (state as u16 & 0x8000) != 0;
                        let was_down = if (vk as usize) < last_states.len() {
                            last_states[vk as usize]
                        } else {
                            false
                        };

                        if is_down && !was_down {
                            let now = Instant::now();
                            let mut deb = debouncer.lock();
                            if deb.should_process(vk, now) {
                                let zone = map_vk_to_zone(vk);
                                let _ = sender.try_send(KeyEvent {
                                    zone,
                                    vk_code: vk,
                                    timestamp: now,
                                });
                            }
                        }
                        if (vk as usize) < last_states.len() {
                            last_states[vk as usize] = is_down;
                        }
                    }

                    // ~83 Hz scan rate (12 ms)
                    std::thread::sleep(Duration::from_millis(12));
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
