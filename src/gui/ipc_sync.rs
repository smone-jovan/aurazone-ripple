//! Non-blocking IPC synchronization worker thread communicating with
//! the AuraZoneRipple background daemon over the Windows Named Pipe.
//!
//! Uses lockless crossbeam channels so the egui UI render loop never
//! blocks, stutters, or drops frames during Named Pipe I/O.

use crossbeam_channel::{bounded, Receiver, Sender};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::ipc::{get_pipe_name, send_command_to_pipe};

/// Commands sent from the UI thread to the IPC background worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuiIpcCommand {
    QueryStatus,
    Toggle,
    Reload,
    Stop,
}

impl GuiIpcCommand {
    pub fn as_str(&self) -> &'static str {
        match self {
            GuiIpcCommand::QueryStatus => "STATUS",
            GuiIpcCommand::Toggle => "TOGGLE",
            GuiIpcCommand::Reload => "RELOAD",
            GuiIpcCommand::Stop => "STOP",
        }
    }
}

/// Status update emitted by the IPC worker back to the UI thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonStatusUpdate {
    pub is_connected: bool,
    pub is_running: bool,
    pub is_active: bool,
    pub preset_name: String,
    pub raw_response: String,
}

impl Default for DaemonStatusUpdate {
    fn default() -> Self {
        Self {
            is_connected: false,
            is_running: false,
            is_active: false,
            preset_name: "Cyberpunk".to_string(),
            raw_response: String::new(),
        }
    }
}

/// Parses the daemon's raw protocol line into a structured `DaemonStatusUpdate`.
pub fn parse_daemon_response(resp: &str, current: &mut DaemonStatusUpdate) {
    let trimmed = resp.trim();
    current.raw_response = trimmed.to_string();

    if trimmed.starts_with("OK running") {
        current.is_connected = true;
        current.is_running = true;

        for part in trimmed.split_whitespace() {
            if let Some(val) = part.strip_prefix("active=") {
                current.is_active = val == "true";
            } else if let Some(val) = part.strip_prefix("preset=") {
                current.preset_name = val.to_string();
            }
        }
    } else if trimmed.starts_with("OK TOGGLE") {
        current.is_connected = true;
        current.is_running = true;
        current.is_active = trimmed.contains("active");
    } else if trimmed.starts_with("OK RELOAD") {
        current.is_connected = true;
        current.is_running = true;
    } else if trimmed.starts_with("OK STOP") {
        current.is_connected = false;
        current.is_running = false;
        current.is_active = false;
    }
}

/// IPC synchronization bridge managing the background worker thread and channels.
pub struct IpcSyncBridge {
    pub tx_cmd: Sender<GuiIpcCommand>,
    pub rx_state: Receiver<DaemonStatusUpdate>,
    running: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl IpcSyncBridge {
    /// Spawns the dedicated non-blocking IPC background worker thread.
    pub fn spawn() -> Self {
        let (tx_cmd, rx_cmd) = bounded::<GuiIpcCommand>(16);
        let (tx_state, rx_state) = bounded::<DaemonStatusUpdate>(16);
        let running = Arc::new(AtomicBool::new(true));
        let worker_running = Arc::clone(&running);

        let handle = thread::Builder::new()
            .name("aurazone_ipc_worker".to_string())
            .spawn(move || {
                let mut status = DaemonStatusUpdate::default();
                let pipe_name = get_pipe_name();

                // Initial status inquiry
                match send_command_to_pipe(&pipe_name, "STATUS") {
                    Ok(resp) => {
                        parse_daemon_response(&resp, &mut status);
                        let _ = tx_state.try_send(status.clone());
                    }
                    Err(_) => {
                        status.is_connected = false;
                        status.is_running = false;
                        let _ = tx_state.try_send(status.clone());
                    }
                }

                while worker_running.load(Ordering::Relaxed) {
                    // Poll for commands from UI or trigger periodic 500ms status heartbeat
                    let maybe_cmd = rx_cmd.recv_timeout(Duration::from_millis(500));
                    let cmd_str = match maybe_cmd {
                        Ok(cmd) => cmd.as_str(),
                        Err(crossbeam_channel::RecvTimeoutError::Timeout) => "STATUS",
                        Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
                    };

                    match send_command_to_pipe(&pipe_name, cmd_str) {
                        Ok(resp) => {
                            parse_daemon_response(&resp, &mut status);
                            let _ = tx_state.try_send(status.clone());
                        }
                        Err(err) => {
                            // If daemon pipe is unreachable, update disconnected state
                            status.is_connected = false;
                            status.is_running = false;
                            status.raw_response = format!("Pipe unreachable: {}", err);
                            let _ = tx_state.try_send(status.clone());
                        }
                    }
                }
            })
            .ok();

        Self {
            tx_cmd,
            rx_state,
            running,
            handle,
        }
    }

    /// Stops the worker thread gracefully.
    pub fn shutdown(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for IpcSyncBridge {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_daemon_response_status() {
        let mut status = DaemonStatusUpdate::default();
        parse_daemon_response("OK running active=true preset=Matrix\n", &mut status);
        assert!(status.is_connected);
        assert!(status.is_running);
        assert!(status.is_active);
        assert_eq!(status.preset_name, "Matrix");

        parse_daemon_response("OK running active=false preset=Stealth\n", &mut status);
        assert!(status.is_connected);
        assert!(status.is_running);
        assert!(!status.is_active);
        assert_eq!(status.preset_name, "Stealth");
    }

    #[test]
    fn test_parse_daemon_response_toggle_and_stop() {
        let mut status = DaemonStatusUpdate::default();
        parse_daemon_response("OK TOGGLE active\n", &mut status);
        assert!(status.is_connected);
        assert!(status.is_active);

        parse_daemon_response("OK TOGGLE paused\n", &mut status);
        assert!(status.is_connected);
        assert!(!status.is_active);

        parse_daemon_response("OK STOP\n", &mut status);
        assert!(!status.is_connected);
        assert!(!status.is_running);
        assert!(!status.is_active);
    }
}
