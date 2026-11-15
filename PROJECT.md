# Project: AuraZoneRipple_Rust

## Architecture
- **Hardware Abstraction Layer (`src/hid/`)**: Direct USB HID interface to ASUS N-Key device (`VID 0x0B05`, `PID 0x19B6`, `Col05`, Report ID `0x5D`, Direct Command `0xBC`, 64-byte buffer).
- **Input Capture & Physics Engine (`src/hook/`, `src/physics/`)**: Win32 `WH_KEYBOARD_LL` low-level hook + `GetAsyncKeyState` async poller, static LUT VK mapping, fixed inline debounce filter, two-phase easing curves, HSV rainbow progression, 50 FPS superposition engine with lockless atomic UI preview (`Arc<[AtomicU32; 8]>`).
- **Windows Named Pipe IPC Subsystem (`src/ipc/`, `src/daemon.rs`, `src/ghelper.rs`)**: Win32 `CreateMutexW` single-instance enforcement, multi-client duplex named pipe server (`\\.\pipe\AuraZoneRipple_IPC`), CLI commands (`--daemon`, `--toggle`, `--status`, `--stop`, `--profile`), G-Helper M5 hotkey integration.
- **Sleek Minimalist Native GUI (`src/gui/`)**: Low-memory `eframe/egui` with `glow` OpenGL backend, clean minimalist dark palette, interactive parameter sliders, preset switcher, user profile 1 & 2 save/apply buttons, and live 4-zone + wraparound lightbar visualizer.
- **G-Helper Surgical In-Place Patcher (`src/patcher.rs`)**: Binary patcher scanning unique 10-byte IL signature in `GHelper.exe` to redirect `brfalse.s 0x3c` to `0x30`, exposing the official "Ripple" mode directly in G-Helper with zero clutter.
- **Lifecycle Auto-Watcher & Hardware Re-Prime (`src/daemon.rs`)**: Watches G-Helper's `config.json` every 300ms. Auto-wakes and re-primes the microcontroller into Direct Mode (`0xBC`) on mode 8 (Ripple); auto-pauses, restores static lighting, and enters zero-packet sleep on mode != 8.
- **Silent Windows System Tray Companion (`src/tray.rs`)**: Win32 `Shell_NotifyIconW` integration bound to `WinSta0\Default` desktop, pinned in taskbar, with right-click menu and single-instance GUI focus.
- **End-to-End Testing & Release Packaging**: 4-Tier requirement test matrix (141 automated tests total, 100% pass rate), standalone release binary (`target/release/aurazone_ripple.exe`) with LTO, strip, and zero non-system DLL dependencies.

## Feature Inventory
| # | Feature | Description | Milestone | Source |
|---|---|---|---|---|
| F1 | ASUS N-Key HID Protocol | Report 0x5D, Command 0xBC, 64-byte direct mode buffer | M1 | Survey / Hardware Spec |
| F2 | Hardware Abstraction Layer | Win32AuraDevice & MockAuraDevice trait implementations | M1 | Survey / Codebase |
| F3 | 12-to-8 Slot Zone Mapping | ASUS physical layout mapping for keyboard & lightbar | M1 | Survey / Codebase |
| F4 | Win32 Named Pipe IPC Server | Duplex pipe server with concurrent client & burst handling | M3 | Survey / ORIGINAL_REQUEST |
| F5 | Single-Instance Daemon Mutex | Win32 CreateMutexW preventing duplicate daemon execution | M3 | Survey / ORIGINAL_REQUEST |
| F6 | CLI Command Router | --daemon, --toggle, --status, --stop, --profile commands | M3 | Survey / ORIGINAL_REQUEST |
| F7 | G-Helper Integration | --install-ghelper binding M5 key, --restore profile color | M3 | Survey / ORIGINAL_REQUEST |
| F8 | Win32 Keyboard Hook & Poller | WH_KEYBOARD_LL + GetAsyncKeyState with UIPI bypass | M2 | Survey / Codebase |
| F9 | Wave Physics & Color Engine | Two-phase easing, 50 FPS superposition, atomic preview | M2 | Survey / Codebase |
| F10 | Minimalist Native GUI | eframe/glow low-memory GUI, dark palette, live visualizer | M4 | Survey / ORIGINAL_REQUEST |
| F11 | E2E Automated Verification | 100% pass across protocol, physics, and e2e_cli test suites | M5 | Survey / TEST_READY |
| F12 | Release Packaging | target/release/aurazone_ripple.exe standalone optimization | M5 | Survey / ORIGINAL_REQUEST |
| F13 | User Custom Profiles | Profil 1 & Profil 2 persistent slots in config, GUI, and CLI | M6 | Survey / User Request |
| F14 | G-Helper Surgical Patcher | In-place 1-byte jump patch to unhide official "Ripple" mode | M7 | Survey / User Request |
| F15 | Lifecycle Auto-Watcher | Config.json watcher with auto-reprime and zero-packet sleep | M7 | Survey / User Request |
| F16 | System Tray Companion | Silent native tray icon, Default desktop binding, single-instance GUI | M8 | Survey / User Request |

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|---|---|---|---|
| 1 | HID Protocol & HAL | crates/aura_zone_ripple (src/hid) | none | DONE |
| 2 | Hook & Physics Engine | src/hook/, src/physics/ | M1 | DONE |
| 3 | Named Pipe IPC & CLI Daemon | src/ipc/, src/daemon.rs, src/ghelper.rs, src/main.rs | M2 | DONE |
| 4 | Minimalist Native GUI | src/gui/ (mod, theme, widgets, ipc_sync), Cargo.toml | M3 | DONE |
| 5 | E2E Verification & Packaging | full test suite execution, target/release binary sign-off | M3, M4 | DONE |
| 6 | User Custom Profiles | src/config.rs, src/gui/, src/main.rs (Profil 1 & 2) | M5 | DONE |
| 7 | G-Helper Patcher & Watcher | src/patcher.rs, src/daemon.rs (mode 8 auto-reprime & unhide) | M6 | DONE |
| 8 | Tray & Desktop Station Binding | src/tray.rs, src/daemon.rs, WinSta0\Default desktop binding | M7 | DONE |

## Interface Contracts
### IPC Server ↔ CLI / GUI Clients
- Pipe Name: `\\.\pipe\AuraZoneRipple_IPC` (or env `AURAZONE_PIPE_NAME` in tests)
- Single-Instance Mutex: `Local\AuraZoneRipple_Daemon_Mutex` (or env `AURAZONE_MUTEX_NAME` in tests)
- Commands: `STATUS`, `TOGGLE`, `STOP`, `RELOAD`
- Response: Standard text lines (`OK running ...`, `OK TOGGLE active/paused`, `OK STOP`, `ERR ...`)

### Physics Engine ↔ GUI Visualizer
- Lockless Atomic Buffer: `Arc<[AtomicU32; 8]>` (`0x00RRGGBB` packed format)
- Zones: 0=WASD, 1=Mid-Left, 2=Mid-Right, 3=Numpad, 4..7=Wraparound Lightbar

### G-Helper ↔ Daemon
- Config Path: `C:\Users\ROG\AppData\Roaming\GHelper\config.json`
- M5 Hotkey Action: `aurazone_ripple.exe --toggle`
- Mode 8 (Ripple): Triggers auto-reprime and active 50 FPS wave rendering.
- Mode != 8: Triggers auto-pause, invokes `restore_static_lighting()`, and enters zero-packet sleep.

## Code Layout
- `src/main.rs` — CLI argument parsing, mode dispatch (daemon, toggle, status, stop, gui, profile, patch-ghelper, unpatch-ghelper)
- `src/lib.rs` — Crate exports (`hid`, `hook`, `physics`, `config`, `ipc`, `daemon`, `ghelper`, `gui`, `patcher`, `tray`)
- `src/ipc/` — Named Pipe IPC server, client, mutex, and protocol definitions
- `src/daemon.rs` — Background service lifecycle orchestrator & G-Helper config watcher
- `src/ghelper.rs` — G-Helper config reader/writer and static profile restoration
- `src/patcher.rs` — G-Helper surgical in-place byte patcher & unpatcher
- `src/tray.rs` — Native Windows system tray icon, desktop binding, and context menu
- `src/gui/` — Minimalist native GUI (`mod.rs`, `theme.rs`, `widgets.rs`, `ipc_sync.rs`)
- `tests/` — Automated test suites (`protocol_tests.rs`, `physics_tests.rs`, `e2e_cli_tests.rs`, `harness/`)
