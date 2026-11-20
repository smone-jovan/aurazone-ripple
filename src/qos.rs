//! Windows 10/11 Scheduling QoS, EcoQoS Throttling Disabler, and 1ms Multimedia Timer Guard.
//!
//! Prevents CPU starvation, timer stretching, and LowLevelHooksTimeout silent unhooking
//! when running alongside 100% CPU/GPU AAA games (Cyberpunk 2077, Black Myth: Wukong, CS2, Valorant).

use std::ffi::c_void;

#[cfg(windows)]
use windows_sys::Win32::Foundation::{BOOL, HANDLE};
#[cfg(windows)]
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentThread, SetPriorityClass, SetThreadPriority,
    ABOVE_NORMAL_PRIORITY_CLASS, THREAD_PRIORITY_HIGHEST, THREAD_PRIORITY_TIME_CRITICAL,
};

/// Win32 `ProcessPowerThrottling` information class for `SetProcessInformation`.
const PROCESS_POWER_THROTTLING: i32 = 4;
/// Win32 `ThreadPowerThrottling` information class for `SetThreadInformation`.
const THREAD_POWER_THROTTLING: i32 = 4;

const PROCESS_POWER_THROTTLING_CURRENT_VERSION: u32 = 1;
const PROCESS_POWER_THROTTLING_EXECUTION_SPEED: u32 = 0x1;
const PROCESS_POWER_THROTTLING_IGNORE_TIMER_RESOLUTION: u32 = 0x4;

#[repr(C)]
struct PowerThrottlingState {
    version: u32,
    control_mask: u32,
    state_mask: u32,
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn SetProcessInformation(
        h_process: HANDLE,
        process_information_class: i32,
        process_information: *const c_void,
        process_information_size: u32,
    ) -> BOOL;

    fn SetThreadInformation(
        h_thread: HANDLE,
        thread_information_class: i32,
        thread_information: *const c_void,
        thread_information_size: u32,
    ) -> BOOL;
}

#[cfg(windows)]
#[link(name = "winmm")]
extern "system" {
    fn timeBeginPeriod(u_period: u32) -> u32;
    fn timeEndPeriod(u_period: u32) -> u32;
}

/// Elevates the daemon process priority to `ABOVE_NORMAL_PRIORITY_CLASS` and explicitly
/// disables Windows 11 EcoQoS (Efficiency Mode) and timer resolution throttling.
pub fn boost_process_for_gaming() {
    #[cfg(windows)]
    unsafe {
        let h_proc = GetCurrentProcess();

        // 1. Elevate process priority class to ABOVE_NORMAL_PRIORITY_CLASS (BasePriority 10)
        // so background daemon threads are never starved by normal background workers.
        let _ = SetPriorityClass(h_proc, ABOVE_NORMAL_PRIORITY_CLASS);

        // 2. Disable Windows 11 EcoQoS (Execution Speed Throttling) & Timer Resolution Ignore
        let throttling = PowerThrottlingState {
            version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
            control_mask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED
                | PROCESS_POWER_THROTTLING_IGNORE_TIMER_RESOLUTION,
            state_mask: 0, // 0 = Explicitly turn OFF throttling (High QoS / Performance cores)
        };

        let _ = SetProcessInformation(
            h_proc,
            PROCESS_POWER_THROTTLING,
            &throttling as *const _ as *const c_void,
            std::mem::size_of::<PowerThrottlingState>() as u32,
        );
    }
}

/// Elevates the calling thread for low-latency input capture and disables per-thread EcoQoS.
///
/// - `time_critical = true`: Sets `THREAD_PRIORITY_TIME_CRITICAL` (used for `WH_KEYBOARD_LL`
///   message pump thread to guarantee `<1ms` wake-up and prevent `LowLevelHooksTimeout`).
/// - `time_critical = false`: Sets `THREAD_PRIORITY_HIGHEST` (used for `GetAsyncKeyStatePoller`
///   and the 50 FPS render loop).
pub fn boost_input_thread(time_critical: bool) {
    #[cfg(windows)]
    unsafe {
        let h_thread = GetCurrentThread();
        let prio = if time_critical {
            THREAD_PRIORITY_TIME_CRITICAL
        } else {
            THREAD_PRIORITY_HIGHEST
        };
        let _ = SetThreadPriority(h_thread, prio);

        let throttling = PowerThrottlingState {
            version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
            control_mask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
            state_mask: 0,
        };

        let _ = SetThreadInformation(
            h_thread,
            THREAD_POWER_THROTTLING,
            &throttling as *const _ as *const c_void,
            std::mem::size_of::<PowerThrottlingState>() as u32,
        );
    }
}

/// RAII guard for Windows Multimedia Timer 1ms resolution (`timeBeginPeriod(1)`).
/// Automatically calls `timeEndPeriod(1)` when dropped.
pub struct HighResTimerGuard {
    period_ms: u32,
    active: bool,
}

impl HighResTimerGuard {
    pub fn new(period_ms: u32) -> Self {
        #[cfg(windows)]
        let active = unsafe { timeBeginPeriod(period_ms) == 0 };
        #[cfg(not(windows))]
        let active = false;

        Self { period_ms, active }
    }
}

impl Drop for HighResTimerGuard {
    fn drop(&mut self) {
        #[cfg(windows)]
        if self.active {
            unsafe {
                let _ = timeEndPeriod(self.period_ms);
            }
        }
    }
}

/// Returns `true` if the current process is running with elevated (Administrator / High Integrity) privileges.
pub fn is_process_elevated() -> bool {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::Security::{
            GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
        };
        use windows_sys::Win32::System::Threading::OpenProcessToken;

        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 || token.is_null() {
            return false;
        }

        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut ret_len: u32 = 0;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut _ as *mut c_void,
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut ret_len,
        );
        CloseHandle(token);

        ok != 0 && elevation.TokenIsElevated != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_process_elevated_does_not_panic() {
        let _ = is_process_elevated();
    }
}
