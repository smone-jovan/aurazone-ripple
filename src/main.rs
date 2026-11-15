//! Minimal CLI entry point router for AuraZoneRipple_Rust.

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "aurazone_ripple",
    version = "0.1.0",
    author = "ROG User",
    about = "ASUS ROG Strix G513RC Aura Zone Ripple Lighting"
)]
pub struct CliArgs {
    /// Run as background daemon process
    #[arg(short, long, conflicts_with = "stop")]
    pub daemon: bool,

    /// Run in mock mode (no physical hardware required)
    #[arg(short, long, action = clap::ArgAction::SetTrue, overrides_with = "mock")]
    pub mock: bool,

    /// Toggle ripple effect state (active / paused)
    #[arg(short, long)]
    pub toggle: bool,

    /// Stop the background daemon
    #[arg(long)]
    pub stop: bool,

    /// Query background daemon status
    #[arg(short, long)]
    pub status: bool,

    /// Restore default static lighting profile
    #[arg(short, long)]
    pub restore: bool,

    /// Install G-Helper M5 custom key action binding
    #[arg(long = "install-ghelper")]
    pub install_ghelper: bool,

    /// Launch sleek minimalist GUI
    #[arg(short, long)]
    pub gui: bool,

    /// Switch to user profile slot (1 or 2)
    #[arg(short, long, value_parser = clap::value_parser!(u8))]
    pub profile: Option<u8>,

    /// Patch G-Helper to unhide official "Ripple" mode in UI dropdown
    #[arg(long = "patch-ghelper")]
    pub patch_ghelper: bool,

    /// Unpatch G-Helper and restore original binary
    #[arg(long = "unpatch-ghelper")]
    pub unpatch_ghelper: bool,

    /// Custom path to GHelper.exe (optional override)
    #[arg(long = "ghelper-path")]
    pub ghelper_path: Option<std::path::PathBuf>,
}

fn spawn_daemon_background(mock: bool) {
    if let Ok(current_exe) = std::env::current_exe() {
        let mut cmd = std::process::Command::new(current_exe);
        cmd.arg("--daemon");
        if mock {
            cmd.arg("--mock");
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            const DETACHED_PROCESS: u32 = 0x00000008;
            cmd.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);
        }
        let _ = cmd.spawn();
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
}

fn main() {
    let args = CliArgs::parse();

    if args.daemon {
        if let Err(e) = aura_zone_ripple::daemon::run_daemon(args.mock) {
            eprintln!("Daemon execution failed: {}", e);
            std::process::exit(1);
        }
    } else if args.status {
        match aura_zone_ripple::ipc::send_ipc_command("STATUS") {
            Ok(resp) => {
                print!("{}", resp);
            }
            Err(_) => {
                println!("AuraZoneRipple is idle/stopped.");
                std::process::exit(1);
            }
        }
    } else if args.toggle {
        match aura_zone_ripple::ipc::send_ipc_command("TOGGLE") {
            Ok(resp) => {
                print!("{}", resp);
            }
            Err(_) => {
                println!("AuraZoneRipple daemon not running. Spawning daemon in background...");
                spawn_daemon_background(args.mock);
                match aura_zone_ripple::ipc::send_ipc_command("STATUS") {
                    Ok(resp) => print!("Daemon started: {}", resp),
                    Err(e) => eprintln!("Daemon started, but pipe query failed: {}", e),
                }
            }
        }
    } else if args.stop {
        match aura_zone_ripple::ipc::send_ipc_command("STOP") {
            Ok(resp) => {
                print!("{}", resp);
            }
            Err(e) => {
                eprintln!("Error connecting to daemon: {}", e);
                std::process::exit(1);
            }
        }
    } else if args.restore {
        match aura_zone_ripple::ghelper::restore_static_lighting(args.mock) {
            Ok(rgb) => {
                println!(
                    "Restored static lighting to RGB({}, {}, {}).",
                    rgb[0], rgb[1], rgb[2]
                );
            }
            Err(e) => {
                eprintln!("Error restoring static lighting: {}", e);
                std::process::exit(1);
            }
        }
    } else if let Some(slot) = args.profile {
        if slot != 1 && slot != 2 {
            eprintln!("Error: Profile slot must be 1 or 2.");
            std::process::exit(1);
        }
        let settings_path = std::env::var("AURAZONE_SETTINGS_PATH")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("ripple_settings.json"));

        let (mut config, _) = aura_zone_ripple::config::RippleConfig::load_or_recover(&settings_path);
        config.apply_profile(slot);
        if let Err(e) = config.save_to_path(&settings_path) {
            eprintln!("Error saving config: {}", e);
            std::process::exit(1);
        }
        println!("Switched to Profil {}.", slot);
        let _ = aura_zone_ripple::ipc::send_ipc_command("RELOAD");
    } else if args.patch_ghelper {
        let exe_path = args.ghelper_path.or_else(aura_zone_ripple::find_ghelper_exe_path);
        match exe_path {
            Some(path) => {
                println!("Menemukan GHelper.exe di: {}", path.display());
                match aura_zone_ripple::patch_ghelper_file(&path) {
                    Ok(_) => {
                        println!("========================================================");
                        println!(" [OK] G-Helper BERHASIL DI-PATCH SECARA AMAN!");
                        println!(" [OK] Mode 'Ripple' resmi kini aktif di dropdown G-Helper!");
                        println!(" [OK] Backup otomatis tersimpan di: {}.bak", path.display());
                        println!(" [OK] Icon Tray AuraZone aktif berdampingan dengan G-Helper.");
                        println!("========================================================");
                        println!("Tips: Pengaturan warna, speed, & profil custom dapat");
                        println!("      diakses kapan saja lewat Icon Tray di taskbar!");
                    }
                    Err(e) => {
                        eprintln!("Gagal mem-patch G-Helper: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            None => {
                eprintln!("Error: GHelper.exe tidak ditemukan secara otomatis.");
                eprintln!("Gunakan: aurazone_ripple.exe --patch-ghelper --ghelper-path \"C:\\Path\\To\\GHelper.exe\"");
                std::process::exit(1);
            }
        }
    } else if args.unpatch_ghelper {
        let exe_path = args.ghelper_path.or_else(aura_zone_ripple::find_ghelper_exe_path);
        match exe_path {
            Some(path) => {
                match aura_zone_ripple::unpatch_ghelper_file(&path) {
                    Ok(_) => {
                        println!("[OK] G-Helper berhasil dikembalikan ke versi original.");
                    }
                    Err(e) => {
                        eprintln!("Gagal unpatch G-Helper: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            None => {
                eprintln!("Error: GHelper.exe tidak ditemukan.");
                std::process::exit(1);
            }
        }
    } else if args.install_ghelper {
        match aura_zone_ripple::ghelper::install_ghelper_binding() {
            Ok(action) => {
                println!("Successfully installed G-Helper M5 action: {}", action);
            }
            Err(e) => {
                eprintln!("Error installing G-Helper binding: {}", e);
                std::process::exit(1);
            }
        }
    } else if args.gui {
        // Auto-start daemon if not running so GUI is fully live on hardware
        if aura_zone_ripple::ipc::send_ipc_command("STATUS").is_err() {
            spawn_daemon_background(args.mock);
        }
        if let Err(e) = aura_zone_ripple::gui::run_gui() {
            eprintln!("Failed to launch GUI: {}", e);
            std::process::exit(1);
        }
    } else {
        println!("=================================================================");
        println!(" ASUS ROG Strix 4-Zone Reactive Ripple Engine (Rust Native)");
        println!(" Hardware: ROG Strix G513RC (ASUS N-Key VID_0B05 PID_19B6)");
        println!("=================================================================");
        println!("Memulai live engine... (Tekan Ctrl + C untuk keluar)");
        if let Err(e) = aura_zone_ripple::daemon::run_daemon(args.mock) {
            eprintln!("Execution failed: {}", e);
            std::process::exit(1);
        }
    }
}
