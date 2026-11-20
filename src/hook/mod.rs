//! Keyboard input capture subsystem: WH_KEYBOARD_LL hook, UIPI fallback poller, and 4-zone mapping.

pub mod poller;
pub mod raw_input;
pub mod vk_map;
pub mod win32_hook;

use std::sync::Arc;
use crossbeam_channel::{Receiver, Sender};
use thiserror::Error;

pub use poller::{PollerHandle, POLLED_KEYS};
pub use raw_input::{is_raw_input_active, normalize_raw_vk, RawInputHandle};
pub use vk_map::{map_vk_to_zone, DebounceFilter, KeyEvent, DEBOUNCE_WINDOW_MS};
pub use win32_hook::KeyboardHookHandle;

#[derive(Error, Debug)]
pub enum HookError {
    #[error("Failed to install WH_KEYBOARD_LL hook (Win32 error: {0})")]
    SetHookFailed(u32),
    #[error("Failed to spawn keyboard thread: {0}")]
    ThreadSpawnFailed(#[from] std::io::Error),
    #[error("Hook initialization timed out")]
    InitTimeout,
}

/// Unified keyboard listener managing the OS low-level hook, Raw Input sink, and UIPI fallback poller threads.
pub struct KeyboardListener {
    hook: Option<KeyboardHookHandle>,
    raw_input: Option<RawInputHandle>,
    poller: Option<PollerHandle>,
    sender: Sender<KeyEvent>,
    receiver: Receiver<KeyEvent>,
    debouncer: Arc<DebounceFilter>,
}

impl KeyboardListener {
    /// Starts the low-level hook, Raw Input (`RIDEV_INPUTSINK`) worker, and 200 Hz fallback poller.
    pub fn start() -> Result<Self, HookError> {
        Self::start_with_options(true, true)
    }

    /// Starts listener with configurable hook/poller options (supports mock/testing and headless mode).
    pub fn start_with_options(enable_hook: bool, enable_poller: bool) -> Result<Self, HookError> {
        let (sender, receiver) = crossbeam_channel::unbounded();
        let debouncer = Arc::new(DebounceFilter::default());

        let hook = if enable_hook {
            Some(KeyboardHookHandle::start(sender.clone(), debouncer.clone())?)
        } else {
            None
        };

        let raw_input = if enable_hook {
            RawInputHandle::start(sender.clone(), debouncer.clone()).ok()
        } else {
            None
        };

        let poller = if enable_poller {
            Some(PollerHandle::start(sender.clone(), debouncer.clone())?)
        } else {
            None
        };

        Ok(Self {
            hook,
            raw_input,
            poller,
            sender,
            receiver,
            debouncer,
        })
    }

    /// Creates a mock keyboard listener with no OS hooks or poller threads (ideal for CI and offline tests).
    pub fn new_mock() -> Self {
        Self::start_with_options(false, false).expect("Mock listener creation cannot fail")
    }

    pub fn receiver(&self) -> Receiver<KeyEvent> {
        self.receiver.clone()
    }

    pub fn sender(&self) -> Sender<KeyEvent> {
        self.sender.clone()
    }

    pub fn debouncer(&self) -> Arc<DebounceFilter> {
        self.debouncer.clone()
    }

    pub fn has_raw_input(&self) -> bool {
        self.raw_input.is_some()
    }

    pub fn stop(&mut self) {
        if let Some(mut raw) = self.raw_input.take() {
            raw.stop();
        }
        if let Some(mut hook) = self.hook.take() {
            hook.stop();
        }
        if let Some(mut poller) = self.poller.take() {
            poller.stop();
        }
    }
}

impl Drop for KeyboardListener {
    fn drop(&mut self) {
        self.stop();
    }
}
