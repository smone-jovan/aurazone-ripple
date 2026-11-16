//! G-Helper binary patcher: in-place surgical branch patch to unhide EXCLUSIVELY "Ripple" mode.
//!
//! Rather than turning on all non-functional PerKey modes (Star, Rain, Laser, Highlight, Comet, Flash),
//! this targeted patch modifies the conditional branch `brfalse.s 0x3c` (jump over all 5 modes)
//! to `brfalse.s 0x30` (jump over Star, Rain, Highlight, Laser, landing directly on Ripple).
//!
//! Result:
//! - Star, Rain, Highlight, Laser -> SKIPPED (not shown in UI)
//! - Ripple -> UNHIDDEN & EXPOSED (modes[Ripple] = "Ripple")
//! - Comet, Flash -> SKIPPED (second if (perKey) remains false)
//! - File size & metadata -> 100% UNCHANGED (zero corruption risk)

use std::path::{Path, PathBuf};
use thiserror::Error;

pub const TARGET_SIGNATURE: [u8; 10] = [
    0x25, 0x2c, 0x3c, 0x06, 0x1a, 0x72, 0x94, 0xd1, 0x00, 0x70,
];

pub const ORIGINAL_BYTES: [u8; 3] = [0x25, 0x2c, 0x3c];
pub const PATCHED_BYTES: [u8; 3] = [0x25, 0x2c, 0x30];

/// 2000.0 ms in IEEE 754 float64 (little-endian)
pub const TIMER_2000_MS: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x9F, 0x40];
/// 100.0 ms in IEEE 754 float64 (little-endian)
pub const TIMER_100_MS: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x59, 0x40];

pub const TIMER_WRITE_SIGNATURE: [u8; 14] = [
    0x7E, 0xBC, 0x05, 0x00, 0x04, 0x23, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x9F, 0x40,
];

pub const TIMER_INIT_SIGNATURE: [u8; 19] = [
    0x73, 0x52, 0x00, 0x00, 0x0A, 0x80, 0x04, 0x00, 0x00, 0x04, 0x23, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x40, 0x9F, 0x40,
];

#[derive(Error, Debug, PartialEq, Eq)]
pub enum PatchError {
    #[error("Signature not found in binary")]
    SignatureNotFound,
    #[error("Multiple conflicting signatures found ({0} occurrences)")]
    AmbiguousSignature(usize),
    #[error("Already patched")]
    AlreadyPatched,
    #[error("Not patched (already in original state)")]
    NotPatched,
    #[error("I/O error: {0}")]
    Io(String),
}

/// Applies the in-place surgical branch patch to an in-memory buffer of `GHelper.exe`.
pub fn patch_ghelper_buffer(data: &mut [u8]) -> Result<usize, PatchError> {
    // 1. Check if already patched
    let mut patched_sig = TARGET_SIGNATURE;
    patched_sig[..3].copy_from_slice(&PATCHED_BYTES);

    let patched_matches: Vec<usize> = data
        .windows(patched_sig.len())
        .enumerate()
        .filter_map(|(i, w)| if w == patched_sig { Some(i) } else { None })
        .collect();

    if !patched_matches.is_empty() {
        return Err(PatchError::AlreadyPatched);
    }

    // 2. Find target signature
    let matches: Vec<usize> = data
        .windows(TARGET_SIGNATURE.len())
        .enumerate()
        .filter_map(|(i, w)| if w == TARGET_SIGNATURE { Some(i) } else { None })
        .collect();

    match matches.len() {
        0 => Err(PatchError::SignatureNotFound),
        1 => {
            let offset = matches[0];
            data[offset..offset + 3].copy_from_slice(&PATCHED_BYTES);
            Ok(offset)
        }
        n => Err(PatchError::AmbiguousSignature(n)),
    }
}

/// Restores the original 3 bytes to an in-memory buffer of `GHelper.exe`.
pub fn unpatch_ghelper_buffer(data: &mut [u8]) -> Result<usize, PatchError> {
    let mut patched_sig = TARGET_SIGNATURE;
    patched_sig[..3].copy_from_slice(&PATCHED_BYTES);

    let matches: Vec<usize> = data
        .windows(patched_sig.len())
        .enumerate()
        .filter_map(|(i, w)| if w == patched_sig { Some(i) } else { None })
        .collect();

    match matches.len() {
        0 => {
            // Check if already original
            if data.windows(TARGET_SIGNATURE.len()).any(|w| w == TARGET_SIGNATURE) {
                Err(PatchError::NotPatched)
            } else {
                Err(PatchError::SignatureNotFound)
            }
        }
        1 => {
            let offset = matches[0];
            data[offset..offset + 3].copy_from_slice(&ORIGINAL_BYTES);
            Ok(offset)
        }
        n => Err(PatchError::AmbiguousSignature(n)),
    }
}

/// Applies surgical in-place modification to G-Helper's AppConfig debounce timer (2000ms -> 100ms).
pub fn patch_debounce_timer_buffer(data: &mut [u8]) -> Result<usize, PatchError> {
    let mut patched_write_sig = TIMER_WRITE_SIGNATURE;
    patched_write_sig[6..14].copy_from_slice(&TIMER_100_MS);

    // Check if already patched
    let is_already_patched = data.windows(patched_write_sig.len()).any(|w| w == patched_write_sig);
    if is_already_patched {
        return Err(PatchError::AlreadyPatched);
    }

    let write_matches: Vec<usize> = data
        .windows(TIMER_WRITE_SIGNATURE.len())
        .enumerate()
        .filter_map(|(i, w)| if w == TIMER_WRITE_SIGNATURE { Some(i) } else { None })
        .collect();

    if write_matches.is_empty() {
        return Err(PatchError::SignatureNotFound);
    }
    if write_matches.len() > 1 {
        return Err(PatchError::AmbiguousSignature(write_matches.len()));
    }

    let write_off = write_matches[0];
    data[write_off + 6..write_off + 14].copy_from_slice(&TIMER_100_MS);

    // Also patch init signature if present
    let init_matches: Vec<usize> = data
        .windows(TIMER_INIT_SIGNATURE.len())
        .enumerate()
        .filter_map(|(i, w)| if w == TIMER_INIT_SIGNATURE { Some(i) } else { None })
        .collect();
    if init_matches.len() == 1 {
        let init_off = init_matches[0];
        data[init_off + 11..init_off + 19].copy_from_slice(&TIMER_100_MS);
    }

    Ok(write_off)
}

/// Restores original 2000ms debounce timer in G-Helper buffer.
pub fn unpatch_debounce_timer_buffer(data: &mut [u8]) -> Result<usize, PatchError> {
    let mut patched_write_sig = TIMER_WRITE_SIGNATURE;
    patched_write_sig[6..14].copy_from_slice(&TIMER_100_MS);

    let write_matches: Vec<usize> = data
        .windows(patched_write_sig.len())
        .enumerate()
        .filter_map(|(i, w)| if w == patched_write_sig { Some(i) } else { None })
        .collect();

    if write_matches.is_empty() {
        if data.windows(TIMER_WRITE_SIGNATURE.len()).any(|w| w == TIMER_WRITE_SIGNATURE) {
            return Err(PatchError::NotPatched);
        } else {
            return Err(PatchError::SignatureNotFound);
        }
    }
    if write_matches.len() > 1 {
        return Err(PatchError::AmbiguousSignature(write_matches.len()));
    }

    let write_off = write_matches[0];
    data[write_off + 6..write_off + 14].copy_from_slice(&TIMER_2000_MS);

    let mut patched_init_sig = TIMER_INIT_SIGNATURE;
    patched_init_sig[11..19].copy_from_slice(&TIMER_100_MS);
    let init_matches: Vec<usize> = data
        .windows(patched_init_sig.len())
        .enumerate()
        .filter_map(|(i, w)| if w == patched_init_sig { Some(i) } else { None })
        .collect();
    if init_matches.len() == 1 {
        let init_off = init_matches[0];
        data[init_off + 11..init_off + 19].copy_from_slice(&TIMER_2000_MS);
    }

    Ok(write_off)
}

/// Auto-discovers the path to `GHelper.exe` on the current machine.
pub fn find_ghelper_exe_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("GHELPER_EXE_PATH") {
        let pb = PathBuf::from(p);
        if pb.is_file() {
            return Some(pb);
        }
    }

    // Common known installation paths
    let known_candidates = [
        r"C:\G-helpe\GHelper\GHelper.exe",
        r"C:\GHelper\GHelper.exe",
        r"C:\Program Files\GHelper\GHelper.exe",
    ];

    for candidate in &known_candidates {
        let p = PathBuf::from(candidate);
        if p.is_file() {
            return Some(p);
        }
    }

    // Check AppData Local Programs
    if let Ok(local_appdata) = std::env::var("LOCALAPPDATA") {
        let p = PathBuf::from(local_appdata).join("Programs").join("GHelper").join("GHelper.exe");
        if p.is_file() {
            return Some(p);
        }
    }

    // Check current working directory
    let cwd_candidate = PathBuf::from("GHelper.exe");
    if cwd_candidate.is_file() {
        return Some(cwd_candidate);
    }

    None
}

/// Safely terminates any running GHelper processes before patching.
pub fn kill_running_ghelper() -> bool {
    #[cfg(windows)]
    {
        use std::process::Command;
        let _ = Command::new("taskkill")
            .args(["/F", "/IM", "GHelper.exe"])
            .output();
        std::thread::sleep(std::time::Duration::from_millis(500));
        true
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Spawns GHelper.exe detached in background.
pub fn spawn_ghelper(path: &Path) -> Result<(), std::io::Error> {
    let mut cmd = std::process::Command::new(path);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        const DETACHED_PROCESS: u32 = 0x00000008;
        cmd.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);
    }
    cmd.spawn()?;
    Ok(())
}

/// Patches the GHelper.exe file on disk with automatic backup.
pub fn patch_ghelper_file(path: &Path) -> Result<(), PatchError> {
    if !path.is_file() {
        return Err(PatchError::Io(format!("File not found: {}", path.display())));
    }

    kill_running_ghelper();

    let mut content = std::fs::read(path)
        .map_err(|e| PatchError::Io(format!("Failed to read binary: {}", e)))?;

    // Create backup ONLY if not existing AND original unpatched signatures are present
    let bak_path = path.with_extension("bak");
    if !bak_path.exists() {
        let has_orig_branch = content.windows(TARGET_SIGNATURE.len()).any(|w| w == TARGET_SIGNATURE);
        let has_orig_timer = content.windows(TIMER_WRITE_SIGNATURE.len()).any(|w| w == TIMER_WRITE_SIGNATURE);
        if has_orig_branch || has_orig_timer {
            let _ = std::fs::copy(path, &bak_path);
        }
    }

    let mode_res = patch_ghelper_buffer(&mut content);
    let timer_res = patch_debounce_timer_buffer(&mut content);

    let mut any_patched = false;
    if mode_res.is_ok() {
        any_patched = true;
    }
    if timer_res.is_ok() {
        any_patched = true;
    }

    if !any_patched {
        let mode_already = mode_res == Err(PatchError::AlreadyPatched);
        let timer_already = timer_res == Err(PatchError::AlreadyPatched);
        if mode_already && timer_already {
            let _ = spawn_ghelper(path);
            return Err(PatchError::AlreadyPatched);
        }
        if mode_res.is_err() && !mode_already {
            let _ = spawn_ghelper(path);
            return Err(mode_res.unwrap_err());
        }
        if timer_res.is_err() && !timer_already {
            let _ = spawn_ghelper(path);
            return Err(timer_res.unwrap_err());
        }
    }

    // Atomic file replacement via temporary file
    let tmp_path = path.with_extension("patched.tmp");
    std::fs::write(&tmp_path, &content)
        .map_err(|e| PatchError::Io(format!("Failed to write temporary patched binary: {}", e)))?;

    if std::fs::rename(&tmp_path, path).is_err() {
        if let Err(e) = std::fs::copy(&tmp_path, path) {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(PatchError::Io(format!("Failed to replace binary: {}", e)));
        }
        let _ = std::fs::remove_file(&tmp_path);
    }

    let _ = spawn_ghelper(path);
    Ok(())
}

/// Unpatches GHelper.exe file on disk, restoring from .bak or reversing the patch.
pub fn unpatch_ghelper_file(path: &Path) -> Result<(), PatchError> {
    if !path.is_file() {
        return Err(PatchError::Io(format!("File not found: {}", path.display())));
    }

    kill_running_ghelper();

    let bak_path = path.with_extension("bak");
    if bak_path.exists() {
        std::fs::copy(&bak_path, path)
            .map_err(|e| PatchError::Io(format!("Failed to restore from backup: {}", e)))?;
    } else {
        let mut content = std::fs::read(path)
            .map_err(|e| PatchError::Io(format!("Failed to read binary: {}", e)))?;

        let _ = unpatch_ghelper_buffer(&mut content);
        let _ = unpatch_debounce_timer_buffer(&mut content);

        let tmp_path = path.with_extension("unpatched.tmp");
        std::fs::write(&tmp_path, &content)
            .map_err(|e| PatchError::Io(format!("Failed to write temporary binary: {}", e)))?;

        if std::fs::rename(&tmp_path, path).is_err() {
            if let Err(e) = std::fs::copy(&tmp_path, path) {
                let _ = std::fs::remove_file(&tmp_path);
                return Err(PatchError::Io(format!("Failed to replace binary: {}", e)));
            }
            let _ = std::fs::remove_file(&tmp_path);
        }
    }

    let _ = spawn_ghelper(path);
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GHelperPatchStatus {
    Patched(PathBuf),
    NeedsPatch(PathBuf),
    NotFound,
    Error(String),
}

/// Inspects GHelper.exe on disk to determine whether it has been patched.
pub fn check_ghelper_patch_state() -> GHelperPatchStatus {
    let path = match find_ghelper_exe_path() {
        Some(p) => p,
        None => return GHelperPatchStatus::NotFound,
    };

    let content = match std::fs::read(&path) {
        Ok(c) => c,
        Err(e) => return GHelperPatchStatus::Error(format!("Failed to read {}: {}", path.display(), e)),
    };

    let mut patched_branch = TARGET_SIGNATURE;
    patched_branch[..3].copy_from_slice(&PATCHED_BYTES);
    let is_branch_patched = content.windows(patched_branch.len()).any(|w| w == patched_branch);

    let mut patched_timer = TIMER_WRITE_SIGNATURE;
    patched_timer[6..14].copy_from_slice(&TIMER_100_MS);
    let is_timer_patched = content.windows(patched_timer.len()).any(|w| w == patched_timer);

    if is_branch_patched && is_timer_patched {
        GHelperPatchStatus::Patched(path)
    } else {
        GHelperPatchStatus::NeedsPatch(path)
    }
}

/// Checks if the Windows Logon Scheduled Task for AuraZoneRipple is registered.
pub fn is_startup_task_installed() -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        std::process::Command::new("schtasks")
            .args(["/Query", "/TN", "AuraZoneRipple"])
            .creation_flags(CREATE_NO_WINDOW)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Registers the Windows Logon Instant Startup Task silently in Task Scheduler.
pub fn install_startup_task() -> Result<(), String> {
    let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_str = current_exe.to_str().ok_or("Invalid executable path")?;

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let output = std::process::Command::new("schtasks")
            .args([
                "/Create", "/F",
                "/TN", "AuraZoneRipple",
                "/SC", "ONLOGON",
                "/TR", &format!("\"{}\" --daemon", exe_str),
                "/RL", "HIGHEST",
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| e.to_string())?;

        if output.status.success() {
            return Ok(());
        }

        // Fallback without /RL HIGHEST if not running with Administrator privileges
        let fallback = std::process::Command::new("schtasks")
            .args([
                "/Create", "/F",
                "/TN", "AuraZoneRipple",
                "/SC", "ONLOGON",
                "/TR", &format!("\"{}\" --daemon", exe_str),
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| e.to_string())?;

        if fallback.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }
    #[cfg(not(windows))]
    {
        Ok(())
    }
}

/// Deletes the Windows Logon Startup Task from Task Scheduler.
pub fn uninstall_startup_task() -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let output = std::process::Command::new("schtasks")
            .args(["/Delete", "/TN", "AuraZoneRipple", "/F"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| e.to_string())?;

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }
    #[cfg(not(windows))]
    {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_patch_and_unpatch_buffer() {
        let mut buffer = Vec::new();
        buffer.extend_from_slice(b"HEADER_PADDING_BYTES_1234567890");
        buffer.extend_from_slice(&TARGET_SIGNATURE);
        buffer.extend_from_slice(b"TAIL_PADDING_BYTES_0987654321");

        // First patch
        let offset = patch_ghelper_buffer(&mut buffer).expect("Patch must succeed");
        assert_eq!(offset, 31);
        assert_eq!(&buffer[offset..offset + 3], &PATCHED_BYTES);

        // Attempting second patch must return AlreadyPatched
        let err = patch_ghelper_buffer(&mut buffer).unwrap_err();
        assert_eq!(err, PatchError::AlreadyPatched);

        // Unpatch
        let unpatch_off = unpatch_ghelper_buffer(&mut buffer).expect("Unpatch must succeed");
        assert_eq!(unpatch_off, 31);
        assert_eq!(&buffer[unpatch_off..unpatch_off + 3], &ORIGINAL_BYTES);

        // Attempting second unpatch must return NotPatched
        let unpatch_err = unpatch_ghelper_buffer(&mut buffer).unwrap_err();
        assert_eq!(unpatch_err, PatchError::NotPatched);
    }

    #[test]
    fn test_signature_not_found() {
        let mut buffer = vec![0u8; 100];
        let err = patch_ghelper_buffer(&mut buffer).unwrap_err();
        assert_eq!(err, PatchError::SignatureNotFound);
    }

    #[test]
    fn test_ambiguous_signature() {
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&TARGET_SIGNATURE);
        buffer.extend_from_slice(b"INTERMEDIATE");
        buffer.extend_from_slice(&TARGET_SIGNATURE);

        let err = patch_ghelper_buffer(&mut buffer).unwrap_err();
        assert_eq!(err, PatchError::AmbiguousSignature(2));
    }

    #[test]
    fn test_patch_and_unpatch_debounce_timer() {
        let mut buffer = Vec::new();
        buffer.extend_from_slice(b"HEADER_PADDING_BYTES_1234567890");
        buffer.extend_from_slice(&TIMER_WRITE_SIGNATURE);
        buffer.extend_from_slice(&TIMER_INIT_SIGNATURE);
        buffer.extend_from_slice(b"TAIL_PADDING_BYTES_0987654321");

        let off = patch_debounce_timer_buffer(&mut buffer).expect("Patch debounce timer must succeed");
        assert_eq!(off, 31);
        assert_eq!(&buffer[off + 6..off + 14], &TIMER_100_MS);

        // Double patch should fail with AlreadyPatched
        let err = patch_debounce_timer_buffer(&mut buffer).unwrap_err();
        assert_eq!(err, PatchError::AlreadyPatched);

        // Unpatch
        let unpatch_off = unpatch_debounce_timer_buffer(&mut buffer).expect("Unpatch must succeed");
        assert_eq!(unpatch_off, 31);
        assert_eq!(&buffer[unpatch_off + 6..unpatch_off + 14], &TIMER_2000_MS);

        // Double unpatch should fail with NotPatched
        let unpatch_err = unpatch_debounce_timer_buffer(&mut buffer).unwrap_err();
        assert_eq!(unpatch_err, PatchError::NotPatched);
    }
}
