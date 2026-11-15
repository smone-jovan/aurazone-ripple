# ⚡ AuraZoneRipple

> **Ultra-Low Memory 4-Zone Reactive Ripple RGB Lighting Extension for ASUS ROG Strix Laptops & G-Helper**  
> *Written in pure native Rust with a sleek minimalist dark GUI, silent system tray companion, and native G-Helper dropdown integration.*

[![Rust](https://img.shields.io/badge/Rust-1.80%2B-orange.svg?logo=rust)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/Platform-Windows%2010%20%2F%2011%20x64-blue.svg?logo=windows)](https://microsoft.com)
[![Hardware](https://img.shields.io/badge/Hardware-ASUS%20ROG%20Strix%20G513-red.svg?logo=asus)](https://rog.asus.com)
[![License](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](#-license)
[![Tests](https://img.shields.io/badge/Tests-142%20passed-brightgreen.svg)](tests/)

---

## 🌟 Overview

Most ASUS ROG laptops equipped with **4-Zone RGB keyboards** (such as the **ROG Strix G513RC**) lack native firmware support for per-key reactive ripple lighting. Official software (Armoury Crate) and lightweight controllers (G-Helper) restrict 4-zone models to static, breathing, or color-cycle modes.

**AuraZoneRipple** brings **real-time reactive ripple typing** to ASUS ROG 4-Zone laptops through a software-driven high-frequency (50 FPS) USB HID direct pipeline.

Unlike heavy RGB suites (SignalRGB, Armoury Crate) that consume hundreds of megabytes of RAM and multiple background services, AuraZoneRipple is written in **native Rust**, consuming **only ~11.7 MB of RAM** and **under 0.1% CPU**.

---

## ✨ Features

- **🌊 4-Zone Keyboard + Front Wraparound Lightbar Waterfall**:
  - Keystrokes in any zone trigger smooth, cosine-eased wave ripples that propagate across the 4 keyboard zones and cascade downward onto the front underglow lightbar.
  - Reverse-engineered motherboard trace inversion fix ensures natural left-to-right propagation matching key positions.
- **🖥️ Sleek Minimalist Native GUI (`egui/eframe/glow`)**:
  - Clean, distraction-free minimalist dark interface with procedural keyboard and front lightbar deck visualizer.
  - Interactive parameter controls for wave speed, decay duration, idle base glow, and waterfall cascade timing.
  - 6 Built-in Presets: *Cyberpunk, Volcano, Matrix, Synthwave, Rainbow Keystrokes, and Stealth*.
  - **Custom User Profiles 1 & 2**: Save, customize, and switch between your own tailored profiles with one click.
- **🎮 Seamless G-Helper Native Dropdown Integration**:
  - **Surgical In-Place Patcher (`--patch-ghelper`)**: Unhides the official "Ripple" mode directly inside G-Helper's keyboard mode dropdown with zero clutter and zero risk of bundle corruption. Also patches G-Helper's internal `AppConfig` disk debounce timer from 2,000 ms to 100 ms.
  - **Ultra-Fast Sub-150ms Lifecycle Auto-Watcher**: Monitors G-Helper profile changes at 50 ms intervals. Mode transitions execute in ~100–150 ms (down from 2.4s).
  - **15-Frame Continuous Direct Mode Lock**: Automatically sends a 15-frame (300 ms) continuous handshake burst upon activation to guarantee Direct Mode (`0xBC`) completely overrides G-Helper's trailing builtin packets.
  - **Pure Zero-Packet Sleep**: When switching back to Static, Breathing, Rainbow, or Strobe in G-Helper, daemon enters a 100% zero-packet state without sending any USB frames, giving G-Helper full, undisturbed control over its built-in modes.
- **🔕 Silent System Tray Companion**:
  - Pinned native tray icon sitting quietly beside G-Helper in the Windows taskbar.
  - Quick right-click menu: Open GUI, Activate Profile 1, Activate Profile 2, Pause/Resume, and Clean Exit.
  - Single-instance window focus protection prevents duplicate GUI instances.
- **🚀 Windows Auto-Startup Support**:
  - Automatically launches into background standby on user logon (`HKCU\...\Run`), running quietly at ~11.7 MB RAM.
- **⚡ Ultra-Low Resource Footprint**:
  - **~11.7 MB Working Set RAM** in background daemon mode (only 2.3 MB private memory).
  - **< 0.1% CPU utilization** using low-level Win32 hooks and asynchronous key polling.
  - Standalone release executable (~4.2 MB) with zero non-system DLL dependencies.
- **🔒 Single-Instance Win32 Mutex & Named Pipe IPC**:
  - Built-in duplex IPC server (`\\.\pipe\AuraZoneRipple_IPC`) handling `--daemon`, `--toggle`, `--status`, `--stop`, `--profile`, and `--gui` commands without race conditions.
- **🐍 Included Reference Python Prototype**:
  - Comes with the complete zero-dependency Python prototype and standalone Tkinter control panel inside `python_prototype/`.

---

## 🔬 Hardware Reverse Engineering Specs

Reverse engineered from ASUS N-Key HID controller communications (`GHelper.USB.Aura` and OpenRGB):

| Parameter | Specification |
|---|---|
| **Vendor ID (VID)** | `0x0B05` (ASUSTeK Computer Inc.) |
| **Product ID (PID)** | `0x19B6` (ASUS N-Key Keyboard / Aura Controller) |
| **Endpoint / Interface** | **`Col05`** (`UsagePage: 0xFF31`, `Usage: 0x0079`) |
| **Report ID** | `0x5D` (Aura Direct Feature Report) |
| **Command Subcode** | `0xBC` (Direct Mode Command) |
| **Buffer Size** | Strictly **64 bytes** (Mandated by Windows HID subsystem) |
| **Handshake Primer** | `[0x5D, 0xBC, 0x01] + [0x00] * 61` |
| **Direct Frame Header** | `[0x5D, 0xBC, 0x00, 0x01, 0x04, 0x00, 0x00, 0x00, 0x00]` |
| **Hardware Slot Map** | `[0, 1, 2, 3, 0, 0, 7, 7, 6, 5, 4, 4]` (Slots 6–11 wire ascending Right $\rightarrow$ Left on ROG Strix G513) |

---

## 💡 Front Wraparound Lightbar Architecture & Hardware Inversion Fix

The ASUS ROG Strix G513 series features a distinctive wraparound LED lightbar lining the front and sides of the laptop chassis. AuraZoneRipple synchronizes this underglow with keyboard typing using a two-stage **Waterfall Cascade Effect**:

### 1. Spatial Topology & Waterfall Flow
When a key is struck, the wave expands horizontally across the 4 keyboard zones and cascades vertically down to the corresponding lightbar segments after a tuned micro-delay (`lightbar_cascade_delay` ~35ms):

```text
       [Zone 0]        [Zone 1]        [Zone 2]        [Zone 3]
       Left/WASD     Center-Left     Center-Right    Right/NumPad
           │               │               │               │
      ─────┴───────────────┴───────────────┴───────────────┴─────  (Keyboard Deck)
           │               │               │               │
           ▼ (Cascade)     ▼ (Cascade)     ▼ (Cascade)     ▼ (Cascade)
      ───────────────────────────────────────────────────────────  (Front Edge)
       [Zone 4]        [Zone 5]        [Zone 6]        [Zone 7]
      Left Underglow  Center-L Under  Center-R Under  Right Underglow
```

### 2. The Motherboard Bus Inversion Gotcha (Reverse Engineering Discovery)
In the 64-byte Direct Mode payload (`Report 0x5D`), 12 RGB slots (36 bytes) are transmitted to the embedded controller (EC).
- Slots `0..=3`: Keyboard Zones 0 to 3
- Slots `4..=5`: Padding / Mirror
- Slots `6..=11`: Front Lightbar LEDs

**The Hardware Inversion Problem:**  
On the ASUS ROG Strix G513 motherboard, physical traces for slots 6 through 11 are routed ascending from **Right to Left** (`Slot 6` is the far-right LED cluster, while `Slot 11` is the far-left LED cluster).

A naive left-to-right mapping `[..., 4, 4, 5, 6, 7, 7]` causes inverted propagation: pressing a key on the left side (WASD) produces light ripples emerging from the right side of the desk!

**The Calibrated Hardware Solution:**  
AuraZoneRipple applies an inverse slot mapping in `src/hid/protocol.rs` to compensate for physical motherboard trace routing:
```rust
pub const ZONE_MAP: [usize; 12] = [0, 1, 2, 3, 0, 0, 7, 7, 6, 5, 4, 4];
```

| Slot Index | Physical Bus Location | Logical Zone Assigned | Desk Underglow Position |
|---|---|---|---|
| `Slot 0..3` | Keyboard Zones 0..3 | `Zone 0..3` | Keyboard Deck Left $\rightarrow$ Right |
| `Slot 4..5` | Reserved / Padding | `Zone 0` | Internal EC Pad |
| `Slot 6` | Motherboard Bus Right-Far | `Zone 7` (Right) | Front Right Edge Underglow |
| `Slot 7` | Motherboard Bus Right-Mid | `Zone 7` (Right) | Front Right-Center Underglow |
| `Slot 8` | Motherboard Bus Center-R | `Zone 6` (Mid-Right) | Front Center-Right Underglow |
| `Slot 9` | Motherboard Bus Center-L | `Zone 5` (Mid-Left) | Front Center-Left Underglow |
| `Slot 10` | Motherboard Bus Left-Mid | `Zone 4` (Left) | Front Left-Center Underglow |
| `Slot 11` | Motherboard Bus Left-Far | `Zone 4` (Left) | Front Left Edge Underglow |

---

## 🚀 Quick Start & Installation

### Option 1: Pre-Built Release Binary
1. Download or locate `aurazone_ripple.exe` in `target/release/`.
2. To unlock the official "Ripple" mode in G-Helper:
   ```cmd
   aurazone_ripple.exe --patch-ghelper
   ```
3. Run the background daemon or launch the GUI:
   ```cmd
   aurazone_ripple.exe --gui
   ```

### Option 2: Build from Source
Ensure you have the Rust toolchain installed ([rustup.rs](https://rustup.rs)):

```bash
# Clone the repository
git clone https://github.com/smone-jovan/aurazone-ripple.git
cd aurazone-ripple

# Run full automated test suite (141 tests)
cargo test

# Compile optimized release binary
cargo build --release
```

The compiled executable will be located at:
```
target/release/aurazone_ripple.exe
```

---

## ⌨️ CLI Usage

```text
ASUS ROG Strix G513RC Aura Zone Ripple Lighting

Usage: aurazone_ripple.exe [OPTIONS]

Options:
  -d, --daemon                       Run as background daemon process (Named Pipe IPC + Tray)
  -m, --mock                         Run in mock mode (no physical hardware required)
  -t, --toggle                       Toggle ripple effect state (active / paused)
      --stop                         Stop the background daemon
  -s, --status                       Query background daemon status
  -r, --restore                      Restore default static lighting profile
      --install-ghelper              Install G-Helper M5 custom key action binding
  -g, --gui                          Launch sleek minimalist GUI control center
  -p, --profile <PROFILE>            Switch to user profile slot (1 or 2)
      --patch-ghelper                Patch G-Helper to unhide official "Ripple" mode in UI dropdown
      --unpatch-ghelper              Unpatch G-Helper and restore original binary
      --ghelper-path <GHELPER_PATH>  Custom path to GHelper.exe (optional override)
  -h, --help                         Print help
  -V, --version                      Print version
```

---

## 📁 Repository Structure

```
aurazone-ripple/
├── src/
│   ├── hid/                 # Win32 HID Col05 direct hardware driver & 64-byte protocol
│   ├── hook/                # Win32 low-level keyboard hook & async key poller
│   ├── physics/             # Two-phase wave easing, superposition, & lightbar cascade
│   ├── ipc/                 # Windows Named Pipe server/client & single-instance mutex
│   ├── gui/                 # Sleek minimalist GUI (egui/eframe/glow) & custom profile widgets
│   ├── patcher.rs           # G-Helper in-place surgical branch patcher & unpatcher
│   ├── tray.rs              # Native Windows system tray icon & context menu
│   ├── config.rs            # ProfileData, Presets, and atomic JSON configuration
│   ├── daemon.rs            # High-performance 50 FPS daemon orchestrator & G-Helper watcher
│   ├── ghelper.rs           # G-Helper M5 binding installer & static profile restoration
│   ├── lib.rs               # Crate library root & public exports
│   └── main.rs              # CLI entry point router
├── tests/                   # Automated 4-tier verification test suite (142 tests)
├── python_prototype/        # Zero-dependency reference Python prototype & control panel
├── LICENSE-MIT              # MIT License
├── LICENSE-APACHE           # Apache 2.0 License
├── Cargo.toml               # Cargo package configuration
└── README.md                # Project documentation
```

---

## 🤝 Acknowledgements

- **[G-Helper](https://github.com/seerge/g-helper)** by `seerge` — The amazing lightweight control tool for ASUS laptops.
- **[OpenRGB](https://gitlab.com/CalcProgrammer1/OpenRGB)** — Hardware documentation for ASUS Aura USB endpoints.

---

## 📄 License

This project is dual-licensed under either:

- **MIT License** ([LICENSE-MIT](LICENSE-MIT))
- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE))

at your option.
