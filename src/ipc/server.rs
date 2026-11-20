//! Win32 Named Pipe IPC Server supporting multi-client concurrency, burst commands,
//! and graceful loopback shutdown signaling.

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_PIPE_CONNECTED, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    FlushFileBuffers, ReadFile, WriteFile, PIPE_ACCESS_DUPLEX,
};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe,
    PIPE_READMODE_MESSAGE, PIPE_TYPE_MESSAGE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};

pub const DEFAULT_PIPE_NAME: &str = r"\\.\pipe\AuraZoneRipple_IPC";

/// RAII wrapper around a Win32 `SECURITY_ATTRIBUTES` with an SDDL security descriptor
/// allowing Medium/Low-integrity GUI and CLI processes to connect to an Elevated daemon pipe.
#[cfg(windows)]
struct PipeSecurityAttributes {
    sd: windows_sys::Win32::Security::PSECURITY_DESCRIPTOR,
    sa: windows_sys::Win32::Security::SECURITY_ATTRIBUTES,
}

#[cfg(windows)]
impl PipeSecurityAttributes {
    fn new_cross_integrity() -> Option<Self> {
        use windows_sys::Win32::Security::Authorization::{
            ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
        };
        use windows_sys::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};

        // Grant Full Access to SYSTEM & Builtin Admins, Generic Read/Write to Authenticated Users,
        // and Low Mandatory Label so standard-user clients can open the pipe under UIPI.
        let sddl: Vec<u16> = "D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;AU)S:(ML;;NW;;;LW)\0"
            .encode_utf16()
            .collect();
        let mut sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        let ok = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut sd,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 || sd.is_null() {
            return None;
        }
        let sa = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: sd,
            bInheritHandle: 0,
        };
        Some(Self { sd, sa })
    }

    fn as_ptr(&self) -> *const windows_sys::Win32::Security::SECURITY_ATTRIBUTES {
        &self.sa as *const _
    }
}

#[cfg(windows)]
impl Drop for PipeSecurityAttributes {
    fn drop(&mut self) {
        if !self.sd.is_null() {
            unsafe {
                windows_sys::Win32::Foundation::LocalFree(self.sd as _);
            }
            self.sd = std::ptr::null_mut();
        }
    }
}

/// Trait defining the daemon IPC command dispatcher.
pub trait IpcCommandHandler: Send + Sync + 'static {
    /// Handles a single command string, returning `(response_string, should_stop)`.
    fn handle_command(&self, cmd: &str) -> (String, bool);

    /// Callback invoked when a STOP command is processed or shutdown is requested.
    fn on_stop(&self);
}

/// Multi-instance Windows Named Pipe IPC Server.
pub struct NamedPipeServer {
    thread_handle: Option<JoinHandle<()>>,
    pipe_name: String,
    running: Arc<AtomicBool>,
}

impl NamedPipeServer {
    /// Resolves the pipe name from `AURAZONE_PIPE_NAME` or default.
    pub fn get_pipe_name() -> String {
        std::env::var("AURAZONE_PIPE_NAME").unwrap_or_else(|_| DEFAULT_PIPE_NAME.to_string())
    }

    /// Starts the Named Pipe listener loop in a background thread.
    pub fn start<H: IpcCommandHandler>(
        pipe_name: String,
        handler: Arc<H>,
        running: Arc<AtomicBool>,
    ) -> Result<Self, std::io::Error> {
        let p_name = pipe_name.clone();
        let r_clone = Arc::clone(&running);

        let thread_handle = thread::Builder::new()
            .name("aurazone-ipc-server".to_string())
            .spawn(move || {
                run_server_loop(p_name, handler, r_clone);
            })?;

        Ok(Self {
            thread_handle: Some(thread_handle),
            pipe_name,
            running,
        })
    }

    /// Gracefully stops the IPC server using loopback wake-up and joins the listener thread.
    pub fn shutdown(&mut self) {
        if self.running.load(Ordering::SeqCst) {
            self.running.store(false, Ordering::SeqCst);
        }
        wake_up_pipe(&self.pipe_name);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for NamedPipeServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// The main listener loop creating and accepting pipe instances.
fn run_server_loop<H: IpcCommandHandler>(
    pipe_name: String,
    handler: Arc<H>,
    running: Arc<AtomicBool>,
) {
    let wide_pipe_name: Vec<u16> = pipe_name.encode_utf16().chain(std::iter::once(0)).collect();
    #[cfg(windows)]
    let security_attrs = PipeSecurityAttributes::new_cross_integrity();
    #[cfg(windows)]
    let sa_ptr = security_attrs
        .as_ref()
        .map(|s| s.as_ptr())
        .unwrap_or(std::ptr::null());

    while running.load(Ordering::SeqCst) {
        let h_pipe = unsafe {
            CreateNamedPipeW(
                wide_pipe_name.as_ptr(),
                PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT,
                PIPE_UNLIMITED_INSTANCES,
                1024,
                1024,
                50,
                sa_ptr,
            )
        };

        if h_pipe == INVALID_HANDLE_VALUE || h_pipe.is_null() {
            if !running.load(Ordering::SeqCst) {
                break;
            }
            thread::sleep(Duration::from_millis(10));
            continue;
        }

        let connected = unsafe { ConnectNamedPipe(h_pipe, std::ptr::null_mut()) };
        let last_err = if connected == 0 {
            unsafe { GetLastError() }
        } else {
            0
        };

        // If shutdown signaled while blocked on ConnectNamedPipe
        if !running.load(Ordering::SeqCst) {
            unsafe {
                DisconnectNamedPipe(h_pipe);
                CloseHandle(h_pipe);
            }
            break;
        }

        if connected != 0 || last_err == ERROR_PIPE_CONNECTED {
            let h_clone = Arc::clone(&handler);
            let r_clone = Arc::clone(&running);
            let pipe_raw = h_pipe as isize;
            thread::spawn(move || {
                let h_pipe = pipe_raw as HANDLE;
                handle_client_pipe(h_pipe, h_clone, r_clone);
            });
        } else {
            unsafe {
                CloseHandle(h_pipe);
            }
        }
    }
}

/// Handles communication for a single client pipe connection.
fn handle_client_pipe<H: IpcCommandHandler>(
    h_pipe: HANDLE,
    handler: Arc<H>,
    running: Arc<AtomicBool>,
) {
    let mut buffer = [0u8; 1024];

    loop {
        let mut bytes_read: u32 = 0;
        let ok = unsafe {
            ReadFile(
                h_pipe,
                buffer.as_mut_ptr() as _,
                buffer.len() as u32,
                &mut bytes_read,
                std::ptr::null_mut(),
            )
        };

        if ok == 0 || bytes_read == 0 {
            break;
        }

        let input_str = String::from_utf8_lossy(&buffer[..bytes_read as usize]);
        let lines: Vec<&str> = input_str
            .lines()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        let mut should_stop_daemon = false;

        if lines.is_empty() {
            let (resp, is_stop) = handler.handle_command("");
            let resp_bytes = resp.as_bytes();
            let mut written: u32 = 0;
            unsafe {
                WriteFile(
                    h_pipe,
                    resp_bytes.as_ptr() as _,
                    resp_bytes.len() as u32,
                    &mut written,
                    std::ptr::null_mut(),
                );
                FlushFileBuffers(h_pipe);
            }
            if is_stop {
                should_stop_daemon = true;
            }
        } else {
            for line in lines {
                let (resp, is_stop) = handler.handle_command(line);
                let resp_bytes = resp.as_bytes();
                let mut written: u32 = 0;
                unsafe {
                    WriteFile(
                        h_pipe,
                        resp_bytes.as_ptr() as _,
                        resp_bytes.len() as u32,
                        &mut written,
                        std::ptr::null_mut(),
                    );
                    FlushFileBuffers(h_pipe);
                }
                if is_stop {
                    should_stop_daemon = true;
                    break;
                }
            }
        }

        if should_stop_daemon {
            running.store(false, Ordering::SeqCst);
            handler.on_stop();
            break;
        }
    }

    unsafe {
        DisconnectNamedPipe(h_pipe);
        CloseHandle(h_pipe);
    }
}

/// Connects to the pipe locally to unblock any pending `ConnectNamedPipe` call on shutdown.
pub fn wake_up_pipe(pipe_name: &str) {
    for _ in 0..10 {
        if let Ok(mut pipe) = OpenOptions::new()
            .read(true)
            .write(true)
            .open(pipe_name)
        {
            let _ = pipe.write_all(b"STOP\n");
            let _ = pipe.flush();
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn test_cross_integrity_security_attributes_creation() {
        let sa = PipeSecurityAttributes::new_cross_integrity()
            .expect("SDDL conversion for cross-integrity Named Pipe must succeed");
        assert!(!sa.as_ptr().is_null());
    }
}
