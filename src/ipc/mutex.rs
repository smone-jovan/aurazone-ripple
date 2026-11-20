//! Win32 Single-Instance Mutex enforcement for AuraZoneRipple daemon.

use thiserror::Error;
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE};
use windows_sys::Win32::System::Threading::CreateMutexW;

pub const DEFAULT_MUTEX_NAME: &str = r"Local\AuraZoneRipple_Daemon_Mutex";

#[derive(Error, Debug)]
pub enum MutexError {
    #[error("Daemon is already running (mutex '{0}' exists)")]
    AlreadyRunning(String),

    #[error("Failed to create Win32 mutex '{name}' (Win32 error: {code})")]
    CreateFailed { name: String, code: u32 },
}

/// Guard holding the Win32 named mutex for the lifetime of the daemon process.
/// Automatically closes the mutex handle on drop.
pub struct SingleInstanceMutex {
    handle: HANDLE,
    name: String,
}

impl SingleInstanceMutex {
    /// Attempts to acquire the single-instance mutex using the name from `AURAZONE_MUTEX_NAME`
    /// or falling back to `DEFAULT_MUTEX_NAME`.
    pub fn acquire() -> Result<Self, MutexError> {
        let name = std::env::var("AURAZONE_MUTEX_NAME")
            .unwrap_or_else(|_| DEFAULT_MUTEX_NAME.to_string());
        Self::acquire_named(&name)
    }

    /// Attempts to acquire the single-instance mutex with a specific name.
    pub fn acquire_named(name: &str) -> Result<Self, MutexError> {
        let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();

        // Call CreateMutexW with bInitialOwner = 0 (false) so unowned until acquired
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, wide.as_ptr()) };

        if handle.is_null() {
            let code = unsafe { GetLastError() };
            if code == 5 {
                // ERROR_ACCESS_DENIED: mutex exists and is owned by an elevated/High-Integrity daemon
                return Err(MutexError::AlreadyRunning(name.to_string()));
            }
            return Err(MutexError::CreateFailed {
                name: name.to_string(),
                code,
            });
        }

        let last_err = unsafe { GetLastError() };
        if last_err == ERROR_ALREADY_EXISTS {
            unsafe {
                CloseHandle(handle);
            }
            return Err(MutexError::AlreadyRunning(name.to_string()));
        }

        Ok(Self {
            handle,
            name: name.to_string(),
        })
    }

    /// Returns the name of the held mutex.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the raw Win32 HANDLE.
    pub fn raw_handle(&self) -> HANDLE {
        self.handle
    }
}

impl Drop for SingleInstanceMutex {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                CloseHandle(self.handle);
            }
            self.handle = std::ptr::null_mut();
        }
    }
}

unsafe impl Send for SingleInstanceMutex {}
unsafe impl Sync for SingleInstanceMutex {}
