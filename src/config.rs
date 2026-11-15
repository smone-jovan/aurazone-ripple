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
    #[serde(alias = "Profil1", alias = "profil_1", alias = "profile_1")]
    Profile1,
    #[serde(alias = "Profil2", alias = "profil_2", alias = "profile_2")]
    Profile2,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileData {
    pub name: String,
    pub speed: f32,
    pub decay: f32,
    pub attack: f32,
    pub lightbar_delay_ms: u64,
    pub rainbow: bool,
    pub color_base: [u8; 3],
    pub color_ripple: [u8; 3],
}

impl Default for ProfileData {
    fn default() -> Self {
        Self {
            name: "Profil 1".to_string(),
            speed: 1.0,
            decay: 0.85,
            attack: 0.15,
            lightbar_delay_ms: 35,
            rainbow: false,
            color_base: [20, 0, 40],       // Deep violet
            color_ripple: [0, 240, 255],    // Bright cyan wave
        }
    }
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
    #[serde(default = "RippleConfig::default_profile_1")]
    pub profile_1: ProfileData,
    #[serde(default = "RippleConfig::default_profile_2")]
    pub profile_2: ProfileData,
}

impl Default for RippleConfig {
    fn default() -> Self {
        Self::from_preset(Preset::Cyberpunk)
    }
}

impl RippleConfig {
    pub fn default_profile_1() -> ProfileData {
        ProfileData {
            name: "Profil 1".to_string(),
            speed: 1.0,
            decay: 0.85,
            attack: 0.15,
            lightbar_delay_ms: 35,
            rainbow: false,
            color_base: [20, 0, 40],       // Deep violet
            color_ripple: [0, 240, 255],    // Bright cyan wave
        }
    }

    pub fn default_profile_2() -> ProfileData {
        ProfileData {
            name: "Profil 2".to_string(),
            speed: 1.1,
            decay: 0.80,
            attack: 0.15,
            lightbar_delay_ms: 35,
            rainbow: false,
            color_base: [40, 20, 0],       // Warm amber
            color_ripple: [255, 180, 0],   // Sunset gold
        }
    }

    pub fn from_preset(preset: Preset) -> Self {
        let p1 = Self::default_profile_1();
        let p2 = Self::default_profile_2();
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
                profile_1: p1,
                profile_2: p2,
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
                profile_1: p1,
                profile_2: p2,
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
                profile_1: p1,
                profile_2: p2,
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
                profile_1: p1,
                profile_2: p2,
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
                profile_1: p1,
                profile_2: p2,
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
                profile_1: p1,
                profile_2: p2,
            },
            Preset::Profile1 => Self {
                preset: Preset::Profile1,
                speed: p1.speed,
                decay: p1.decay,
                attack: p1.attack,
                lightbar_delay_ms: p1.lightbar_delay_ms,
                rainbow: p1.rainbow,
                color_base: p1.color_base,
                color_ripple: p1.color_ripple,
                profile_1: p1,
                profile_2: p2,
            },
            Preset::Profile2 => Self {
                preset: Preset::Profile2,
                speed: p2.speed,
                decay: p2.decay,
                attack: p2.attack,
                lightbar_delay_ms: p2.lightbar_delay_ms,
                rainbow: p2.rainbow,
                color_base: p2.color_base,
                color_ripple: p2.color_ripple,
                profile_1: p1,
                profile_2: p2,
            },
        }
    }

    /// Switches preset while preserving customized user profile slots.
    pub fn switch_preset(&mut self, preset: Preset) {
        match preset {
            Preset::Profile1 => self.apply_profile(1),
            Preset::Profile2 => self.apply_profile(2),
            p => {
                let p1 = self.profile_1.clone();
                let p2 = self.profile_2.clone();
                *self = Self::from_preset(p);
                self.profile_1 = p1;
                self.profile_2 = p2;
            }
        }
    }

    /// Applies stored parameters from the given user profile slot (1 or 2).
    pub fn apply_profile(&mut self, slot: u8) {
        match slot {
            1 => {
                self.preset = Preset::Profile1;
                self.speed = self.profile_1.speed;
                self.decay = self.profile_1.decay;
                self.attack = self.profile_1.attack;
                self.lightbar_delay_ms = self.profile_1.lightbar_delay_ms;
                self.rainbow = self.profile_1.rainbow;
                self.color_base = self.profile_1.color_base;
                self.color_ripple = self.profile_1.color_ripple;
            }
            2 => {
                self.preset = Preset::Profile2;
                self.speed = self.profile_2.speed;
                self.decay = self.profile_2.decay;
                self.attack = self.profile_2.attack;
                self.lightbar_delay_ms = self.profile_2.lightbar_delay_ms;
                self.rainbow = self.profile_2.rainbow;
                self.color_base = self.profile_2.color_base;
                self.color_ripple = self.profile_2.color_ripple;
            }
            _ => {}
        }
    }

    /// Saves the current effect parameters to the given user profile slot (1 or 2).
    pub fn save_current_to_profile(&mut self, slot: u8) {
        let name = if slot == 1 {
            self.profile_1.name.clone()
        } else {
            self.profile_2.name.clone()
        };
        let data = ProfileData {
            name,
            speed: self.speed,
            decay: self.decay,
            attack: self.attack,
            lightbar_delay_ms: self.lightbar_delay_ms,
            rainbow: self.rainbow,
            color_base: self.color_base,
            color_ripple: self.color_ripple,
        };
        match slot {
            1 => {
                self.profile_1 = data;
                self.preset = Preset::Profile1;
            }
            2 => {
                self.profile_2 = data;
                self.preset = Preset::Profile2;
            }
            _ => {}
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

/// Returns the canonical path to the settings file (`%APPDATA%\AuraZoneRipple\ripple_settings.json`).
///
/// 1. Honors `AURAZONE_SETTINGS_PATH` env var if set (for tests/custom overrides).
/// 2. Uses `%APPDATA%\AuraZoneRipple\ripple_settings.json` on Windows.
///    Automatically migrates any existing settings from known locations so custom profiles are preserved.
/// 3. Fallback: next to the current executable or relative "ripple_settings.json".
pub fn get_canonical_settings_path() -> std::path::PathBuf {
    if let Ok(p) = std::env::var("AURAZONE_SETTINGS_PATH") {
        return std::path::PathBuf::from(p);
    }

    if let Ok(appdata) = std::env::var("APPDATA") {
        let dir = std::path::PathBuf::from(appdata).join("AuraZoneRipple");
        let _ = std::fs::create_dir_all(&dir);
        let canonical_file = dir.join("ripple_settings.json");

        if !canonical_file.exists() {
            let candidates = [
                std::path::PathBuf::from(r"C:\Users\ROG\ripple_settings.json"),
                std::path::PathBuf::from(r"C:\Users\ROG\AuraZoneRipple_Rust\ripple_settings.json"),
            ];
            for cand in &candidates {
                if cand.is_file() {
                    let _ = std::fs::copy(cand, &canonical_file);
                    break;
                }
            }
        }
        return canonical_file;
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            return parent.join("ripple_settings.json");
        }
    }

    std::path::PathBuf::from("ripple_settings.json")
}
