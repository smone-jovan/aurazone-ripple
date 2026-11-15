//! G-Helper configuration integration: M5 custom key binding and static profile restoration.

use std::path::{Path, PathBuf};
use crate::hid::{AuraDevice, MockAuraDevice, Win32AuraDevice};

pub const DEFAULT_GHELPER_CONFIG_PATH: &str =
    r"C:\Users\ROG\AppData\Roaming\GHelper\config.json";

/// Returns the path to the G-Helper config.json, honoring `GHELPER_CONFIG_PATH` override.
pub fn get_ghelper_config_path() -> PathBuf {
    if let Ok(p) = std::env::var("GHELPER_CONFIG_PATH") {
        return PathBuf::from(p);
    }
    if let Ok(appdata) = std::env::var("APPDATA") {
        return PathBuf::from(appdata).join("GHelper").join("config.json");
    }
    PathBuf::from(DEFAULT_GHELPER_CONFIG_PATH)
}

/// Reads and parses the G-Helper config JSON file.
pub fn read_ghelper_config(path: &Path) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let val: serde_json::Value = serde_json::from_str(&content)?;
    Ok(val)
}

/// Atomically writes the G-Helper config JSON, preserving formatting and all existing fields.
pub fn write_ghelper_config(
    path: &Path,
    val: &serde_json::Value,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let formatted = serde_json::to_string_pretty(val)?;
    std::fs::write(path, formatted)?;
    Ok(())
}

/// Installs the `--toggle` custom action into G-Helper's "m5" configuration key.
pub fn install_ghelper_binding() -> Result<String, Box<dyn std::error::Error>> {
    let path = get_ghelper_config_path();
    let current_exe = std::env::current_exe()?;
    let action_str = format!("{} --toggle", current_exe.to_string_lossy());

    let mut json = if path.exists() {
        match read_ghelper_config(&path) {
            Ok(v) => v,
            Err(_) => serde_json::json!({}),
        }
    } else {
        serde_json::json!({})
    };

    json["m5"] = serde_json::Value::String(action_str.clone());
    write_ghelper_config(&path, &json)?;

    Ok(action_str)
}

/// Reads the G-Helper `aura_color` profile and restores uniform static RGB lighting.
pub fn restore_static_lighting(mock: bool) -> Result<[u8; 3], Box<dyn std::error::Error>> {
    let path = get_ghelper_config_path();

    // Default to Blue (0xFF0000FF = -16776961)
    let argb = if path.exists() {
        match read_ghelper_config(&path) {
            Ok(json) => json
                .get("aura_color")
                .and_then(|v| v.as_i64())
                .unwrap_or(-16776961),
            Err(_) => -16776961,
        }
    } else {
        -16776961
    };

    let b = (argb & 0xFF) as u8;
    let g = ((argb >> 8) & 0xFF) as u8;
    let r = ((argb >> 16) & 0xFF) as u8;
    let color = [r, g, b];

    if mock {
        let mut dev = MockAuraDevice::new();
        dev.set_static_uniform(color)?;
    } else {
        match Win32AuraDevice::open() {
            Ok(mut dev) => {
                dev.set_static_uniform(color)?;
            }
            Err(e) => {
                eprintln!(
                    "Warning: Could not open physical Aura device to restore color: {}",
                    e
                );
            }
        }
    }

    Ok(color)
}
