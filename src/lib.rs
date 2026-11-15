//! AuraZoneRipple_Rust: Low-memory native reactive 4-zone RGB ripple lighting for ASUS ROG Strix G513RC.

pub mod config;
pub mod daemon;
pub mod ghelper;
pub mod gui;
pub mod hid;
pub mod hook;
pub mod ipc;
pub mod patcher;
pub mod physics;
pub mod tray;

pub use config::{ConfigError, Preset, ProfileData, RippleConfig};
pub use daemon::run_daemon;
pub use patcher::{
    find_ghelper_exe_path, patch_ghelper_buffer, patch_ghelper_file, unpatch_ghelper_buffer,
    unpatch_ghelper_file, PatchError, ORIGINAL_BYTES, PATCHED_BYTES, TARGET_SIGNATURE,
};
pub use gui::run_gui;
pub use ghelper::{
    get_ghelper_config_path, install_ghelper_binding, read_ghelper_config, restore_static_lighting,
    write_ghelper_config,
};
pub use hid::{AuraDevice, AuraError, MockAuraDevice, SharedMockAuraDevice, Win32AuraDevice};
pub use hook::{
    map_vk_to_zone, DebounceFilter, HookError, KeyEvent, KeyboardHookHandle, KeyboardListener,
    PollerHandle, DEBOUNCE_WINDOW_MS, POLLED_KEYS,
};
pub use ipc::{
    get_pipe_name, send_command_to_pipe, send_ipc_command, wake_up_pipe, IpcCommand,
    IpcCommandHandler, IpcResponse, MutexError, NamedPipeServer, SingleInstanceMutex,
};
pub use physics::{
    advance_rainbow, blend_colors, calculate_zone_delay, calculate_zone_intensity,
    calculate_zone_intensity_full, hsv_norm_to_rgb, hsv_to_rgb, linear_distance_attenuation,
    run_engine_loop, saturating_add_rgb, two_phase_easing, ActiveRipple, RippleEngine,
    ATTENUATION_FACTOR, ATTENUATION_PER_ZONE, DEFAULT_RIPPLE_DURATION_MS, DEFAULT_SPREAD_MS,
    DEFAULT_WAVE_DURATION_MS, FRAME_INTERVAL_MS, LIGHTBAR_CASCADE_DELAY_MS,
    LIGHTBAR_INTENSITY_SCALE, MAX_ACTIVE_RIPPLES, RAINBOW_HUE_STEP,
};

// Re-exports and dependency roots for downstream milestones (M2: crossbeam, M3: serde)
pub use crossbeam_channel as _crossbeam_channel;
pub use serde as _serde;
pub use serde_json as _serde_json;
