# Project: AuraZoneRipple_Rust

## Architecture
- **Hardware Abstraction Layer (`src/hid/`)**: Direct USB HID interface to ASUS N-Key device (`VID 0x0B05`, `PID 0x19B6`, `Col05`, Report ID `0x5D`, Direct Command `0xBC`, 64-byte buffer).
- **Input Capture & Physics Engine (`src/hook/`, `src/physics/`)**: Win32 `WH_KEYBOARD_LL` low-level hook + `GetAsyncKeyState` async poller, static LUT VK mapping, fixed inline debounce filter, two-phase easing curves, HSV rainbow progression, 50 FPS superposition engine with lockless atomic UI preview (`Arc<[AtomicU32; 8]>`).
- **Windows Named Pipe IPC Subsystem (`src/ipc/`, `src/daemon.rs`, `src/ghelper.rs`)**: Win32 `CreateMutexW` single-instance enforcement, multi-client duplex named pipe server (`\\.\pipe\AuraZoneRipple_IPC`), CLI commands (`--daemon`, `--toggle`, `--status`, `--stop`), G-Helper M5 hotkey integration.
- **LenovoRGB-Themed Native GUI (`src/gui/`)**: Low-memory `eframe/egui` with `glow` OpenGL backend, dark cyberpunk/stealth palette, interactive parameter sliders, preset switcher, and live 4-zone + wraparound lightbar visualizer.
- **End-to-End Testing & Release Packaging**: 4-Tier requirement test matrix (123 scenarios across 126 test functions, 133 automated tests total), standalone release binary (`target/release/aurazone_ripple.exe`) with LTO, strip, and zero non-system DLL dependencies.

## Feature Inventory
| # | Feature | Description | Milestone | Source |
|---|---|---|---|---|
| F1 | ASUS N-Key HID Protocol | Report 0x5D, Command 0xBC, 64-byte direct mode buffer | M1 | Survey / Hardware Spec |
| F2 | Hardware Abstraction Layer | Win32AuraDevice & MockAuraDevice trait implementations | M1 | Survey / Codebase |
| F3 | 12-to-8 Slot Zone Mapping | ASUS physical layout mapping for keyboard & lightbar | M1 | Survey / Codebase |
| F4 | Win32 Named Pipe IPC Server | Duplex pipe server with concurrent client & burst handling | M3 | Survey / ORIGINAL_REQUEST |
| F5 | Single-Instance Daemon Mutex | Win32 CreateMutexW preventing duplicate daemon execution | M3 | Survey / ORIGINAL_REQUEST |
| F6 | CLI Command Router | --daemon, --toggle, --status, --stop commands | M3 | Survey / ORIGINAL_REQUEST |
| F7 | G-Helper Integration | --install-ghelper binding M5 key, --restore profile color | M3 | Survey / ORIGINAL_REQUEST |
| F8 | Win32 Keyboard Hook & Poller | WH_KEYBOARD_LL + GetAsyncKeyState with UIPI bypass | M2 | Survey / Codebase |
| F9 | Wave Physics & Color Engine | Two-phase easing, 50 FPS superposition, atomic preview | M2 | Survey / Codebase |
| F10 | LenovoRGB Native GUI | eframe/glow low-memory GUI, dark palette, live visualizer | M4 | Survey / ORIGINAL_REQUEST |
| F11 | E2E Automated Verification | 100% pass across protocol, physics, and e2e_cli test suites | M5 | Survey / TEST_READY |
| F12 | Release Packaging | target/release/aurazone_ripple.exe standalone optimization | M5 | Survey / ORIGINAL_REQUEST |

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|---|---|---|---|
| 1 | HID Protocol & HAL | crates/aura_zone_ripple (src/hid) | none | DONE |
| 2 | Hook & Physics Engine | src/hook/, src/physics/ | M1 | DONE |
| 3 | Named Pipe IPC & CLI Daemon | src/ipc/, src/daemon.rs, src/ghelper.rs, src/main.rs | M2 | DONE |
| 4 | LenovoRGB-Themed Native GUI | src/gui/ (mod, theme, widgets, ipc_sync), Cargo.toml | M3 | DONE |
| 5 | E2E Verification & Packaging | full test suite execution, target/release binary sign-off | M3, M4 | DONE |

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
- Static Color Restore: Reads `aura_color` (ARGB) and restores uniform static RGB on exit/restore.

## Code Layout
- `src/main.rs` — CLI argument parsing, mode dispatch (daemon, toggle, status, stop, gui, install-ghelper, restore)
- `src/lib.rs` — Crate exports (`hid`, `hook`, `physics`, `config`, `ipc`, `daemon`, `ghelper`, `gui`)
- `src/ipc/` — Named Pipe IPC server, client, mutex, and protocol definitions
- `src/daemon.rs` — Background service lifecycle orchestrator
- `src/ghelper.rs` — G-Helper config reader/writer
- `src/gui/` — LenovoRGB native GUI (`mod.rs`, `theme.rs`, `widgets.rs`, `ipc_sync.rs`)
- `tests/` — Automated test suites (`protocol_tests.rs`, `physics_tests.rs`, `e2e_cli_tests.rs`, `harness/`)
