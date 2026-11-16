//! Keyboard input capture subsystem: WH_KEYBOARD_LL hook, UIPI fallback poller, and 4-zone mapping.

pub mod poller;
pub mod vk_map;
pub mod win32_hook;

use std::sync::Arc;
use crossbeam_channel::{Receiver, Sender};
use thiserror::Error;

pub use poller::{PollerHandle, POLLED_KEYS};
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

/// Unified keyboard listener managing the OS low-level hook and UIPI fallback poller threads.
pub struct KeyboardListener {
    hook: Option<KeyboardHookHandle>,
    poller: Option<PollerHandle>,
    sender: Sender<KeyEvent>,
    receiver: Receiver<KeyEvent>,
    debouncer: Arc<DebounceFilter>,
}

impl KeyboardListener {
    /// Starts both the low-level hook and UIPI fallback poller.
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

        let poller = if enable_poller {
            Some(PollerHandle::start(sender.clone(), debouncer.clone())?)
        } else {
            None
        };

        Ok(Self {
            hook,
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

    pub fn stop(&mut self) {
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
