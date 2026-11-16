//! Windows System Tray Icon integration for AuraZoneRipple.
//!
//! Creates a lightweight native tray icon in the Windows notification area
//! adjacent to G-Helper, allowing users to:
//! - View running status
//! - Click to launch the native minimalist GUI
//! - Switch directly between Profil 1 and Profil 2
//! - Toggle pause/resume
//! - Exit the background daemon cleanly

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[cfg(windows)]
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
#[cfg(windows)]
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE,
    NOTIFYICONDATAW,
};
#[cfg(windows)]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    DispatchMessageW, GetCursorPos, LoadIconW, PostQuitMessage, RegisterClassW,
    SetForegroundWindow, TrackPopupMenu, TranslateMessage, HMENU, IDI_APPLICATION,
    MF_SEPARATOR, MF_STRING, MSG, TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_RIGHTBUTTON, WM_APP,
    WM_COMMAND, WM_DESTROY, WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_RBUTTONUP, WNDCLASSW,
    WS_OVERLAPPEDWINDOW,
};

pub const WM_TRAYICON: u32 = WM_APP + 101;
pub const WM_POWERBROADCAST: u32 = 0x0218;
pub const PBT_APMRESUMEAUTOMATIC: usize = 0x0012;
pub const PBT_APMRESUMESUSPEND: usize = 0x0007;

pub const ID_TRAY_OPEN_GUI: usize = 2001;
pub const ID_TRAY_PROFILE_1: usize = 2002;
pub const ID_TRAY_PROFILE_2: usize = 2003;
pub const ID_TRAY_TOGGLE: usize = 2004;
pub const ID_TRAY_EXIT: usize = 2005;

static TRAY_REPRIME: parking_lot::RwLock<Option<Arc<AtomicBool>>> = parking_lot::RwLock::new(None);

pub struct TrayIconManager {
    running: Arc<AtomicBool>,
    active: Arc<AtomicBool>,
    _need_reprime: Arc<AtomicBool>,
}

impl TrayIconManager {
    pub fn new(running: Arc<AtomicBool>, active: Arc<AtomicBool>, need_reprime: Arc<AtomicBool>) -> Self {
        *TRAY_REPRIME.write() = Some(Arc::clone(&need_reprime));
        Self {
            running,
            active,
            _need_reprime: need_reprime,
        }
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    /// Spawns the dedicated Windows message pump thread for the system tray icon.
    pub fn spawn_thread(self) -> std::thread::JoinHandle<()> {
        std::thread::Builder::new()
            .name("AuraZone_TrayThread".to_string())
            .spawn(move || {
                #[cfg(windows)]
                unsafe {
                    self.run_tray_loop();
                }
            })
            .expect("Failed to spawn tray icon thread")
    }

    #[cfg(windows)]
    unsafe fn run_tray_loop(&self) {
        use windows_sys::Win32::System::StationsAndDesktops::{OpenDesktopW, SetThreadDesktop};

        // Attach to the user's interactive 'Default' desktop so the tray icon and window messages bind to Explorer
        let default_name: Vec<u16> = "Default\0".encode_utf16().collect();
        let hdesk = OpenDesktopW(default_name.as_ptr(), 0, 0, 0x1FF);
        if hdesk != 0 as _ {
            let _ = SetThreadDesktop(hdesk);
        }

        let class_name: Vec<u16> = "AuraZone_TrayClass\0".encode_utf16().collect();
        let h_instance = windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(std::ptr::null());

        let wnd_class = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(tray_window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: h_instance,
            hIcon: LoadIconW(0 as _, IDI_APPLICATION),
            hCursor: 0 as _,
            hbrBackground: 0 as _,
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };

        let _atom = RegisterClassW(&wnd_class);

        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            class_name.as_ptr(),
            WS_OVERLAPPEDWINDOW,
            0,
            0,
            0,
            0,
            0 as _,
            0 as _,
            h_instance,
            std::ptr::null(),
        );

        if hwnd == 0 as _ {
            return;
        }

        // Setup notification icon data
        let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        nid.uCallbackMessage = WM_TRAYICON;
        nid.hIcon = LoadIconW(0 as _, IDI_APPLICATION);

        let tip = "⚡ AuraZone Ripple (ASUS ROG)\0";
        for (i, c) in tip.encode_utf16().take(127).enumerate() {
            nid.szTip[i] = c;
        }

        // Retry loop in case Explorer taskbar is busy initializing
        for _ in 0..10 {
            if Shell_NotifyIconW(NIM_ADD, &nid) != 0 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(250));
        }

        // Win32 Message Loop with non-blocking peek so thread exits cleanly when daemon stops
        let mut msg: MSG = std::mem::zeroed();
        while self.running.load(Ordering::Relaxed) {
            use windows_sys::Win32::UI::WindowsAndMessaging::{PeekMessageW, PM_REMOVE};
            while PeekMessageW(&mut msg, hwnd, 0, 0, PM_REMOVE) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        Shell_NotifyIconW(NIM_DELETE, &nid);
        DestroyWindow(hwnd);
    }
}

#[cfg(windows)]
unsafe extern "system" fn tray_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_TRAYICON => {
            let event = lparam as u32;
            if event == WM_RBUTTONUP {
                show_tray_menu(hwnd);
            } else if event == WM_LBUTTONDBLCLK || event == WM_LBUTTONUP {
                spawn_gui_process();
            }
            0
        }
        WM_COMMAND => {
            let cmd_id = (wparam & 0xFFFF) as usize;
            match cmd_id {
                ID_TRAY_OPEN_GUI => spawn_gui_process(),
                ID_TRAY_PROFILE_1 => {
                    let _ = std::process::Command::new(std::env::current_exe().unwrap_or_default())
                        .args(["--profile", "1"])
                        .spawn();
                }
                ID_TRAY_PROFILE_2 => {
                    let _ = std::process::Command::new(std::env::current_exe().unwrap_or_default())
                        .args(["--profile", "2"])
                        .spawn();
                }
                ID_TRAY_TOGGLE => {
                    let _ = crate::ipc::send_ipc_command("TOGGLE");
                }
                ID_TRAY_EXIT => {
                    let _ = crate::ipc::send_ipc_command("STOP");
                    PostQuitMessage(0);
                }
                _ => {}
            }
            0
        }
        WM_POWERBROADCAST => {
            let event = wparam as usize;
            if event == PBT_APMRESUMEAUTOMATIC || event == PBT_APMRESUMESUSPEND {
                if let Some(reprime) = TRAY_REPRIME.read().as_ref() {
                    reprime.store(true, Ordering::SeqCst);
                }
            }
            1
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

#[cfg(windows)]
unsafe fn show_tray_menu(hwnd: HWND) {
    let mut pt: POINT = std::mem::zeroed();
    GetCursorPos(&mut pt);

    let hmenu: HMENU = CreatePopupMenu();

    let item_gui: Vec<u16> = "⚡ Buka Pengaturan GUI\0".encode_utf16().collect();
    let item_p1: Vec<u16> = "★ Aktifkan Profil 1\0".encode_utf16().collect();
    let item_p2: Vec<u16> = "★ Aktifkan Profil 2\0".encode_utf16().collect();
    let item_toggle: Vec<u16> = "⏸ Pause / Resume Ripple\0".encode_utf16().collect();
    let item_exit: Vec<u16> = "✖ Tutup AuraZone Daemon\0".encode_utf16().collect();

    AppendMenuW(hmenu, MF_STRING, ID_TRAY_OPEN_GUI, item_gui.as_ptr());
    AppendMenuW(hmenu, MF_SEPARATOR, 0, std::ptr::null());
    AppendMenuW(hmenu, MF_STRING, ID_TRAY_PROFILE_1, item_p1.as_ptr());
    AppendMenuW(hmenu, MF_STRING, ID_TRAY_PROFILE_2, item_p2.as_ptr());
    AppendMenuW(hmenu, MF_STRING, ID_TRAY_TOGGLE, item_toggle.as_ptr());
    AppendMenuW(hmenu, MF_SEPARATOR, 0, std::ptr::null());
    AppendMenuW(hmenu, MF_STRING, ID_TRAY_EXIT, item_exit.as_ptr());

    SetForegroundWindow(hwnd);
    TrackPopupMenu(
        hmenu,
        TPM_BOTTOMALIGN | TPM_LEFTALIGN | TPM_RIGHTBUTTON,
        pt.x,
        pt.y,
        0,
        hwnd,
        std::ptr::null(),
    );
    DestroyMenu(hmenu);
}

fn spawn_gui_process() {
    if let Ok(exe) = std::env::current_exe() {
        let mut cmd = std::process::Command::new(exe);
        cmd.arg("--gui");
        let _ = cmd.spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tray_constants() {
        assert_eq!(ID_TRAY_OPEN_GUI, 2001);
        assert_eq!(ID_TRAY_PROFILE_1, 2002);
        assert_eq!(ID_TRAY_PROFILE_2, 2003);
        assert_eq!(ID_TRAY_TOGGLE, 2004);
        assert_eq!(ID_TRAY_EXIT, 2005);
    }
}
