//! Background daemon orchestrator for AuraZoneRipple_Rust.
//!
//! Wires single-instance mutex, keyboard listener hook/poller, 50 FPS superposition
//! ripple engine, and Windows Named Pipe IPC server.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use parking_lot::RwLock;

use crate::config::RippleConfig;
use crate::hid::{AuraDevice, MockAuraDevice, Win32AuraDevice};
use crate::hook::KeyboardListener;
use crate::ipc::{
    wake_up_pipe, IpcCommand, IpcCommandHandler, IpcResponse, NamedPipeServer, SingleInstanceMutex,
};
use crate::physics::{
    advance_rainbow, RippleEngine, DEFAULT_SPREAD_MS, DEFAULT_WAVE_DURATION_MS,
    FRAME_INTERVAL_MS, LIGHTBAR_CASCADE_DELAY_MS,
};

static GLOBAL_RUNNING: AtomicBool = AtomicBool::new(true);

#[cfg(windows)]
extern "system" {
    fn SetConsoleCtrlHandler(
        handler_routine: Option<unsafe extern "system" fn(u32) -> i32>,
        add: i32,
    ) -> i32;
}

#[cfg(windows)]
unsafe extern "system" fn console_ctrl_handler(ctrl_type: u32) -> i32 {
    if ctrl_type == 0 || ctrl_type == 1 {
        // Explicit Ctrl+C or Ctrl+Break
        GLOBAL_RUNNING.store(false, Ordering::SeqCst);
        1
    } else {
        // Ignore terminal close / detach so daemon persists in system tray
        1
    }
}

/// IPC command handler bound to the running daemon's state.
struct DaemonCommandHandler {
    running: Arc<AtomicBool>,
    active: Arc<AtomicBool>,
    config: Arc<RwLock<RippleConfig>>,
    settings_path: PathBuf,
    pipe_name: String,
}

impl IpcCommandHandler for DaemonCommandHandler {
    fn handle_command(&self, cmd: &str) -> (String, bool) {
        let command = IpcCommand::parse(cmd);
        match command {
            IpcCommand::Status => {
                let is_active = self.active.load(Ordering::SeqCst);
                let preset_name = format!("{:?}", self.config.read().preset);
                let resp = IpcResponse::Status {
                    active: is_active,
                    preset: preset_name,
                };
                (resp.to_protocol_string(), false)
            }
            IpcCommand::Toggle => {
                let was_active = self.active.fetch_xor(true, Ordering::SeqCst);
                let now_active = !was_active;
                let resp = IpcResponse::Toggle { active: now_active };
                (resp.to_protocol_string(), false)
            }
            IpcCommand::Stop => (IpcResponse::Stop.to_protocol_string(), true),
            IpcCommand::Reload => {
                let (new_cfg, _) = RippleConfig::load_or_recover(&self.settings_path);
                *self.config.write() = new_cfg;
                (IpcResponse::Reload.to_protocol_string(), false)
            }
            IpcCommand::Unknown(_) => (
                IpcResponse::Error("Unknown command".to_string()).to_protocol_string(),
                false,
            ),
        }
    }

    fn on_stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        wake_up_pipe(&self.pipe_name);
    }
}

/// Runs the AuraZoneRipple background daemon.
pub fn run_daemon(mock: bool) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Acquire single-instance mutex
    let mutex = match SingleInstanceMutex::acquire() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error acquiring single-instance mutex: {}", e);
            std::process::exit(1);
        }
    };

    #[cfg(windows)]
    unsafe {
        SetConsoleCtrlHandler(Some(console_ctrl_handler), 1);
    }

    // 2. Resolve settings path & load initial configuration
    let settings_path = std::env::var("AURAZONE_SETTINGS_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("ripple_settings.json"));
    let (initial_config, _) = RippleConfig::load_or_recover(&settings_path);
    let shared_config = Arc::new(RwLock::new(initial_config));

    // 3. Initialize Aura HID Device (physical or mock)
    let mut device: Box<dyn AuraDevice> = if mock {
        Box::new(MockAuraDevice::new())
    } else {
        match Win32AuraDevice::open() {
            Ok(dev) => {
                println!("[OK] Kontroler Aura Direct siap pada hardware (Col05).");
                Box::new(dev)
            }
            Err(e) => {
                eprintln!(
                    "Warning: Failed to open physical Aura device ({:?}), falling back to mock device.",
                    e
                );
                Box::new(MockAuraDevice::new())
            }
        }
    };

    // 4. Initialize Keyboard Listener (hook + poller or mock)
    let mut listener = if mock {
        KeyboardListener::new_mock()
    } else {
        match KeyboardListener::start() {
            Ok(lis) => lis,
            Err(e) => {
                eprintln!(
                    "Warning: Failed to start keyboard hook ({:?}), falling back to mock listener.",
                    e
                );
                KeyboardListener::new_mock()
            }
        }
    };
    let rx = listener.receiver();

    // 5. Shared daemon state
    let ghelper_cfg_path = crate::ghelper::get_ghelper_config_path();
    let initial_ghelper_mode = if ghelper_cfg_path.exists() {
        std::fs::read_to_string(&ghelper_cfg_path)
            .ok()
            .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
            .and_then(|j| j.get("aura_mode").and_then(|v| v.as_i64()))
    } else {
        None
    };

    // If G-Helper is active and not on mode 8 (Ripple), start PAUSED to not interfere with G-Helper
    let start_active = match initial_ghelper_mode {
        Some(8) => true,
        Some(_) => false,
        None => true,
    };

    let running = Arc::new(AtomicBool::new(true));
    let active = Arc::new(AtomicBool::new(start_active));
    let ui_preview: Arc<[AtomicU32; 8]> = Arc::new(std::array::from_fn(|_| AtomicU32::new(0)));

    // 6. Start Named Pipe IPC Server
    let pipe_name = NamedPipeServer::get_pipe_name();
    let handler = Arc::new(DaemonCommandHandler {
        running: Arc::clone(&running),
        active: Arc::clone(&active),
        config: Arc::clone(&shared_config),
        settings_path,
        pipe_name: pipe_name.clone(),
    });

    let mut server = NamedPipeServer::start(pipe_name.clone(), handler, Arc::clone(&running))?;

    // 6a. Spawn Windows System Tray Icon thread
    let _tray_thread = crate::tray::TrayIconManager::new(Arc::clone(&running), Arc::clone(&active))
        .spawn_thread();

    // 6b. Spawn G-Helper Config Watcher thread (Auto-Nyala saat Ripple (8), Auto-Pause saat mode lain)
    let ghelper_active = Arc::clone(&active);
    let ghelper_running = Arc::clone(&running);
    let ghelper_path = crate::ghelper::get_ghelper_config_path();
    let need_reprime = Arc::new(AtomicBool::new(true));
    let need_reprime_watcher = Arc::clone(&need_reprime);

    let _ghelper_watcher = thread::Builder::new()
        .name("AuraZone_GHelperWatcher".to_string())
        .spawn(move || {
            let mut last_seen_mode: Option<i64> = None;
            while ghelper_running.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(50));
                if ghelper_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&ghelper_path) {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                            if let Some(mode) = json.get("aura_mode").and_then(|v| v.as_i64()) {
                                if Some(mode) != last_seen_mode {
                                    last_seen_mode = Some(mode);
                                    if mode == 8 {
                                        // G-Helper switched to Ripple mode!
                                        ghelper_active.store(true, Ordering::SeqCst);
                                        need_reprime_watcher.store(true, Ordering::SeqCst);
                                        println!("[G-Helper] Mode Ripple (8) aktif -> AuraZone Auto-Nyala & Reprime.");
                                    } else {
                                        // G-Helper switched to another mode (Static, Breathe, Color Cycle, etc.)
                                        if ghelper_active.load(Ordering::SeqCst) {
                                            ghelper_active.store(false, Ordering::SeqCst);
                                            println!("[G-Helper] Mode {} aktif -> AuraZone Auto-Pause.", mode);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

    // 7. 50 FPS Superposition Engine Loop
    let mut engine = RippleEngine::new();
    let mut current_hue = 0.0f32;
    let cascade = Duration::from_millis(LIGHTBAR_CASCADE_DELAY_MS);
    let frame_interval = Duration::from_millis(FRAME_INTERVAL_MS);
    let mut was_active = false;
    let mut reprime_countdown: u32 = 0;

    while running.load(Ordering::Relaxed) && GLOBAL_RUNNING.load(Ordering::Relaxed) {
        let frame_start = Instant::now();
        let is_active = active.load(Ordering::Relaxed);
        let cfg = shared_config.read().clone();

        // If daemon is paused (e.g. G-Helper is in Static, Breathe, Color Cycle mode):
        // NEVER send ANY frames to hardware! Pure zero-packet state lets G-Helper control lighting cleanly.
        if !is_active {
            was_active = false;
            reprime_countdown = 0;
            for (z, c) in [cfg.color_base; 8].iter().enumerate() {
                let u32_val = ((c[0] as u32) << 16) | ((c[1] as u32) << 8) | (c[2] as u32);
                ui_preview[z].store(u32_val, Ordering::Relaxed);
            }
            thread::sleep(Duration::from_millis(20));
            continue;
        }

        if !was_active || need_reprime.swap(false, Ordering::SeqCst) {
            // Re-prime microcontroller into Direct Mode 0xBC over a 300 ms window (15 frames)
            // to guarantee Direct Mode overrides G-Helper's trailing 0xB4 builtin packets
            reprime_countdown = 15;
        }
        if reprime_countdown > 0 {
            reprime_countdown -= 1;
            let handshake = crate::hid::encode_handshake();
            let _ = device.send_feature(&handshake);
        }
        was_active = true;

        let spread = Duration::from_millis(DEFAULT_SPREAD_MS);
        let speed_factor = if cfg.speed > 0.05 { cfg.speed } else { 1.0 };
        let duration =
            Duration::from_millis((DEFAULT_WAVE_DURATION_MS as f32 / speed_factor) as u64);

        // 7a. Drain incoming keystroke events from channel
        while let Ok(event) = rx.try_recv() {
            let zone_names = ["Zona 0 (Kiri/WASD)", "Zona 1 (Tengah-Kiri)", "Zona 2 (Tengah-Kanan)", "Zona 3 (Kanan/Numpad)"];
            let z_name = zone_names.get(event.zone).unwrap_or(&"Unknown");
            println!(" -> [KETIK] Terdeteksi di {}! Gelombang merambat...", z_name);
            let color = if cfg.rainbow {
                advance_rainbow(&mut current_hue)
            } else {
                cfg.color_ripple
            };
            engine.add_ripple_with_duration(event.zone, color, duration);
        }

        // 7b. Prune expired waves and compute frame colors
        let zone_colors = {
            engine.prune_expired(frame_start, spread, cascade);
            engine.render_at_time(frame_start, cfg.color_base, spread, cascade)
        };

        // 7c. Update lockless UI preview buffer (0x00RRGGBB)
        for (z, c) in zone_colors.iter().enumerate() {
            let u32_val = ((c[0] as u32) << 16) | ((c[1] as u32) << 8) | (c[2] as u32);
            ui_preview[z].store(u32_val, Ordering::Relaxed);
        }

        // 7d. Dispatch colors to physical / mock Aura device
        let _ = device.apply_zones(&zone_colors);

        // 7e. Maintain 50 FPS frame timing
        let elapsed = frame_start.elapsed();
        if elapsed < frame_interval {
            thread::sleep(frame_interval - elapsed);
        }
    }

    // 8. Graceful shutdown cleanup
    let _ = device.set_static_uniform([0, 0, 0]);
    listener.stop();
    server.shutdown();
    drop(mutex);

    Ok(())
}
