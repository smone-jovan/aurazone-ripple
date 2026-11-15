//! G-Helper binary patcher: in-place 8-byte patch to unhide official "Ripple" mode.
//!
//! Replaces the `BacklightType == PerKey` check in `Aura.GetModes()` with an unconditional
//! true (`ldc.i4.1` + 7 `nop`s), making G-Helper expose "Ripple" in its UI dropdown
//! without altering file size or corrupting single-file bundle offsets.

use std::path::{Path, PathBuf};
use thiserror::Error;

pub const TARGET_SIGNATURE: [u8; 21] = [
    0x28, 0x85, 0x04, 0x00, 0x06, 0x19, 0xfe, 0x01, 0x28, 0x85, 0x04, 0x00, 0x06, 0x18, 0xfe, 0x01,
    0x0b, 0x25, 0x07, 0x60, 0x0c,
];

pub const ORIGINAL_BYTES: [u8; 8] = [0x28, 0x85, 0x04, 0x00, 0x06, 0x19, 0xfe, 0x01];
pub const PATCHED_BYTES: [u8; 8] = [0x17, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

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

/// Applies the in-place 8-byte patch to an in-memory buffer of `GHelper.exe`.
pub fn patch_ghelper_buffer(data: &mut [u8]) -> Result<usize, PatchError> {
    // 1. Check if already patched
    let mut patched_sig = TARGET_SIGNATURE;
    patched_sig[..8].copy_from_slice(&PATCHED_BYTES);

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
            data[offset..offset + 8].copy_from_slice(&PATCHED_BYTES);
            Ok(offset)
        }
        n => Err(PatchError::AmbiguousSignature(n)),
    }
}

/// Restores the original 8 bytes to an in-memory buffer of `GHelper.exe`.
pub fn unpatch_ghelper_buffer(data: &mut [u8]) -> Result<usize, PatchError> {
    let mut patched_sig = TARGET_SIGNATURE;
    patched_sig[..8].copy_from_slice(&PATCHED_BYTES);

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
            data[offset..offset + 8].copy_from_slice(&ORIGINAL_BYTES);
            Ok(offset)
        }
        n => Err(PatchError::AmbiguousSignature(n)),
    }
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

    // Create backup if not already existing
    let bak_path = path.with_extension("bak");
    if !bak_path.exists() {
        std::fs::copy(path, &bak_path)
            .map_err(|e| PatchError::Io(format!("Failed to create backup: {}", e)))?;
    }

    let mut content = std::fs::read(path)
        .map_err(|e| PatchError::Io(format!("Failed to read binary: {}", e)))?;

    patch_ghelper_buffer(&mut content)?;

    std::fs::write(path, content)
        .map_err(|e| PatchError::Io(format!("Failed to write patched binary: {}", e)))?;

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

        unpatch_ghelper_buffer(&mut content)?;

        std::fs::write(path, content)
            .map_err(|e| PatchError::Io(format!("Failed to write unpatched binary: {}", e)))?;
    }

    let _ = spawn_ghelper(path);
    Ok(())
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
        assert_eq!(&buffer[offset..offset + 8], &PATCHED_BYTES);

        // Attempting second patch must return AlreadyPatched
        let err = patch_ghelper_buffer(&mut buffer).unwrap_err();
        assert_eq!(err, PatchError::AlreadyPatched);

        // Unpatch
        let unpatch_off = unpatch_ghelper_buffer(&mut buffer).expect("Unpatch must succeed");
        assert_eq!(unpatch_off, 31);
        assert_eq!(&buffer[unpatch_off..unpatch_off + 8], &ORIGINAL_BYTES);

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
}
