//! Configuration management, presets, and atomic persistence for AuraZoneRipple_Rust.

use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Preset {
    Cyberpunk,
    Matrix,
    Stealth,
    Volcano,
    Synthwave,
    Rainbow,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RippleConfig {
    pub preset: Preset,
    pub speed: f32,
    pub decay: f32,
    pub attack: f32,
    pub lightbar_delay_ms: u64,
    pub rainbow: bool,
    pub color_base: [u8; 3],
    pub color_ripple: [u8; 3],
}

impl Default for RippleConfig {
    fn default() -> Self {
        Self::from_preset(Preset::Cyberpunk)
    }
}

impl RippleConfig {
    pub fn from_preset(preset: Preset) -> Self {
        match preset {
            Preset::Cyberpunk => Self {
                preset: Preset::Cyberpunk,
                speed: 1.0,
                decay: 0.85,
                attack: 0.15,
                lightbar_delay_ms: 35,
                rainbow: false,
                color_base: [255, 0, 128],    // Neon pink
                color_ripple: [0, 210, 255],   // Electric cyan
            },
            Preset::Matrix => Self {
                preset: Preset::Matrix,
                speed: 1.2,
                decay: 0.80,
                attack: 0.15,
                lightbar_delay_ms: 35,
                rainbow: false,
                color_base: [0, 20, 0],       // Deep green
                color_ripple: [0, 255, 65],    // Matrix green
            },
            Preset::Stealth => Self {
                preset: Preset::Stealth,
                speed: 1.0,
                decay: 0.85,
                attack: 0.15,
                lightbar_delay_ms: 35,
                rainbow: false,
                color_base: [0, 0, 0],         // Completely off
                color_ripple: [255, 255, 255], // Crisp white
            },
            Preset::Volcano => Self {
                preset: Preset::Volcano,
                speed: 0.9,
                decay: 0.85,
                attack: 0.15,
                lightbar_delay_ms: 35,
                rainbow: false,
                color_base: [40, 0, 0],
                color_ripple: [255, 60, 0],
            },
            Preset::Synthwave => Self {
                preset: Preset::Synthwave,
                speed: 1.0,
                decay: 0.85,
                attack: 0.15,
                lightbar_delay_ms: 35,
                rainbow: false,
                color_base: [30, 0, 60],
                color_ripple: [255, 0, 128],
            },
            Preset::Rainbow => Self {
                preset: Preset::Rainbow,
                speed: 1.0,
                decay: 0.85,
                attack: 0.15,
                lightbar_delay_ms: 35,
                rainbow: true,
                color_base: [10, 10, 10],
                color_ripple: [255, 0, 0],
            },
        }
    }

    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let config = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn save_to_path<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Fault-recovery loader: If JSON is corrupt, backs up file to `.json.bak` and restores default.
    pub fn load_or_recover<P: AsRef<Path>>(path: P) -> (Self, bool) {
        let path = path.as_ref();
        if !path.exists() {
            let def = Self::default();
            let _ = def.save_to_path(path);
            return (def, false);
        }
        match Self::load_from_path(path) {
            Ok(cfg) => (cfg, false),
            Err(_) => {
                let bak_path = path.with_extension("json.bak");
                let _ = std::fs::copy(path, &bak_path);
                let def = Self::default();
                let _ = def.save_to_path(path);
                (def, true)
            }
        }
    }
}
