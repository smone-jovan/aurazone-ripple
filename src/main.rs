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

    /// Launch modern Cyberpunk GUI
    #[arg(short, long)]
    pub gui: bool,
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
