#![windows_subsystem = "windows"]
//! Minimal CLI and GUI entry point router for AuraZoneRipple_Rust.

use clap::Parser;
use std::io::Write;

#[cfg(windows)]
fn attach_parent_console() {
    use windows_sys::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

#[cfg(not(windows))]
fn attach_parent_console() {}

fn cli_print(msg: &str) {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Console::{GetStdHandle, WriteConsoleW, STD_OUTPUT_HANDLE};
        use windows_sys::Win32::Storage::FileSystem::WriteFile;
        let h = GetStdHandle(STD_OUTPUT_HANDLE);
        let invalid = windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
        if !h.is_null() && h != invalid {
            let wide: Vec<u16> = msg.encode_utf16().collect();
            let mut written = 0;
            if WriteConsoleW(
                h,
                wide.as_ptr() as _,
                wide.len() as u32,
                &mut written,
                std::ptr::null_mut(),
            ) != 0
            {
                return;
            }
            let bytes = msg.as_bytes();
            let mut bytes_written = 0;
            if WriteFile(
                h,
                bytes.as_ptr() as _,
                bytes.len() as u32,
                &mut bytes_written,
                std::ptr::null_mut(),
            ) != 0
            {
                return;
            }
        }
    }
    print!("{}", msg);
}

fn cli_println(msg: &str) {
    cli_print(&format!("{}\r\n", msg));
}

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
        if let Ok(pipe) = std::env::var("AURAZONE_PIPE_NAME") {
            cmd.env("AURAZONE_PIPE_NAME", pipe);
        }
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());
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
    std::panic::set_hook(Box::new(|info| {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let p = std::path::PathBuf::from(appdata).join("AuraZoneRipple").join("panic.log");
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
                let _ = writeln!(f, "PANIC: {:?}", info);
            }
        }
    }));

    let args = CliArgs::parse();

    if args.status
        || args.toggle
        || args.stop
        || args.restore
        || args.patch_ghelper
        || args.unpatch_ghelper
        || args.install_ghelper
        || args.profile.is_some()
    {
        attach_parent_console();
    }

    if args.daemon {
        if let Err(e) = aura_zone_ripple::daemon::run_daemon(args.mock) {
            cli_println(&format!("Daemon execution failed: {}", e));
            std::process::exit(1);
        }
    } else if args.status {
        match aura_zone_ripple::ipc::send_ipc_command("STATUS") {
            Ok(resp) => {
                cli_print(&resp);
            }
            Err(_) => {
                cli_println("AuraZoneRipple is idle/stopped.");
                std::process::exit(1);
            }
        }
    } else if args.toggle {
        match aura_zone_ripple::ipc::send_ipc_command("TOGGLE") {
            Ok(resp) => {
                cli_print(&resp);
            }
            Err(e) => {
                cli_println(&format!("Error connecting to daemon: {}", e));
                std::process::exit(1);
            }
        }
    } else if args.stop {
        match aura_zone_ripple::ipc::send_ipc_command("STOP") {
            Ok(resp) => {
                cli_print(&resp);
            }
            Err(e) => {
                cli_println(&format!("Error connecting to daemon: {}", e));
                std::process::exit(1);
            }
        }
    } else if args.restore {
        match aura_zone_ripple::ghelper::restore_static_lighting(args.mock) {
            Ok(rgb) => {
                cli_println(&format!(
                    "Restored static lighting to RGB({}, {}, {}).",
                    rgb[0], rgb[1], rgb[2]
                ));
            }
            Err(e) => {
                cli_println(&format!("Error restoring static lighting: {}", e));
                std::process::exit(1);
            }
        }
    } else if let Some(slot) = args.profile {
        if slot != 1 && slot != 2 {
            cli_println("Error: Profile slot must be 1 or 2.");
            std::process::exit(1);
        }
        let settings_path = aura_zone_ripple::config::get_canonical_settings_path();

        let (mut config, _) = aura_zone_ripple::config::RippleConfig::load_or_recover(&settings_path);
        config.apply_profile(slot);
        if let Err(e) = config.save_to_path(&settings_path) {
            cli_println(&format!("Error saving config: {}", e));
            std::process::exit(1);
        }
        cli_println(&format!("Switched to Profil {}.", slot));
        let _ = aura_zone_ripple::ipc::send_ipc_command("RELOAD");
    } else if args.patch_ghelper {
        let exe_path = args.ghelper_path.or_else(aura_zone_ripple::find_ghelper_exe_path);
        match exe_path {
            Some(path) => {
                cli_println(&format!("Menemukan GHelper.exe di: {}", path.display()));
                match aura_zone_ripple::patch_ghelper_file(&path) {
                    Ok(_) => {
                        cli_println("========================================================");
                        cli_println(" [OK] G-Helper BERHASIL DI-PATCH SECARA AMAN!");
                        cli_println(" [OK] Mode 'Ripple' resmi kini aktif di dropdown G-Helper!");
                        cli_println(&format!(" [OK] Backup otomatis tersimpan di: {}.bak", path.display()));
                        cli_println(" [OK] Icon Tray AuraZone aktif berdampingan dengan G-Helper.");
                        cli_println("========================================================");
                        cli_println("Tips: Pengaturan warna, speed, & profil custom dapat");
                        cli_println("      diakses kapan saja lewat Icon Tray di taskbar!");
                    }
                    Err(e) => {
                        cli_println(&format!("Gagal mem-patch G-Helper: {}", e));
                        std::process::exit(1);
                    }
                }
            }
            None => {
                cli_println("Error: GHelper.exe tidak ditemukan secara otomatis.");
                cli_println("Gunakan: aurazone_ripple.exe --patch-ghelper --ghelper-path \"C:\\Path\\To\\GHelper.exe\"");
                std::process::exit(1);
            }
        }
    } else if args.unpatch_ghelper {
        let exe_path = args.ghelper_path.or_else(aura_zone_ripple::find_ghelper_exe_path);
        match exe_path {
            Some(path) => {
                match aura_zone_ripple::unpatch_ghelper_file(&path) {
                    Ok(_) => {
                        cli_println("[OK] G-Helper berhasil dikembalikan ke versi original.");
                    }
                    Err(e) => {
                        cli_println(&format!("Gagal unpatch G-Helper: {}", e));
                        std::process::exit(1);
                    }
                }
            }
            None => {
                cli_println("Error: GHelper.exe tidak ditemukan.");
                std::process::exit(1);
            }
        }
    } else if args.install_ghelper {
        match aura_zone_ripple::ghelper::install_ghelper_binding() {
            Ok(action) => {
                cli_println(&format!("Successfully installed G-Helper M5 action: {}", action));
            }
            Err(e) => {
                cli_println(&format!("Error installing G-Helper binding: {}", e));
                std::process::exit(1);
            }
        }
    } else if args.gui {
        // Auto-start daemon if not running so GUI is fully live on hardware
        if aura_zone_ripple::ipc::send_ipc_command("STATUS").is_err() {
            spawn_daemon_background(args.mock);
        }
        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::System::StationsAndDesktops::{OpenDesktopW, SetThreadDesktop};
            use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, SetForegroundWindow};
            let default_name: Vec<u16> = "Default\0".encode_utf16().collect();
            let hdesk = OpenDesktopW(default_name.as_ptr(), 0, 0, 0x1FF);
            if hdesk != 0 as _ {
                SetThreadDesktop(hdesk);
            }
            let title: Vec<u16> = "AuraZoneRipple — Aura Control Center\0".encode_utf16().collect();
            let existing_hwnd = FindWindowW(std::ptr::null(), title.as_ptr());
            if existing_hwnd != 0 as _ {
                SetForegroundWindow(existing_hwnd);
                return;
            }
        }
        if let Err(e) = aura_zone_ripple::gui::run_gui() {
            eprintln!("Failed to launch GUI: {}", e);
            std::process::exit(1);
        }
    } else {
        // Headless test bypass
        if std::env::var("AURAZONE_HEADLESS").is_ok() {
            cli_println("Headless mode: GUI window creation skipped.");
            return;
        }

        // Double-clicked or launched without arguments in Windows: open the GUI!
        if aura_zone_ripple::ipc::send_ipc_command("STATUS").is_err() {
            spawn_daemon_background(args.mock);
        }
        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::System::StationsAndDesktops::{
                CloseDesktop, OpenDesktopW, SetThreadDesktop,
            };
            use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, SetForegroundWindow};
            let default_name: Vec<u16> = "Default\0".encode_utf16().collect();
            let hdesk = OpenDesktopW(default_name.as_ptr(), 0, 0, 0x1FF);
            if hdesk != 0 as _ {
                if SetThreadDesktop(hdesk) == 0 {
                    CloseDesktop(hdesk);
                }
            }
            let title: Vec<u16> = "AuraZoneRipple — Aura Control Center\0".encode_utf16().collect();
            let existing_hwnd = FindWindowW(std::ptr::null(), title.as_ptr());
            if existing_hwnd != 0 as _ {
                SetForegroundWindow(existing_hwnd);
                if hdesk != 0 as _ {
                    CloseDesktop(hdesk);
                }
                return;
            }
        }
        if let Err(e) = aura_zone_ripple::gui::run_gui() {
            eprintln!("Failed to launch GUI: {}", e);
            std::process::exit(1);
        }
    }
}
