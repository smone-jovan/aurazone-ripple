# ⚡ AuraZoneRipple

> **Ultra-Low Memory 4-Zone Reactive Ripple RGB Lighting Extension for ASUS ROG Strix Laptops & G-Helper**  
> *Written in pure native Rust with a modern dark cyberpunk GUI.*

[![Rust](https://img.shields.io/badge/Rust-1.80%2B-orange.svg?logo=rust)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/Platform-Windows%2010%20%2F%2011%20x64-blue.svg?logo=windows)](https://microsoft.com)
[![Hardware](https://img.shields.io/badge/Hardware-ASUS%20ROG%20Strix%20G513-red.svg?logo=asus)](https://rog.asus.com)
[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

---

## 🌟 Overview

Most ASUS ROG laptops equipped with **4-Zone RGB keyboards** (such as the **ROG Strix G513RC**) lack native firmware support for per-key reactive ripple lighting. Official software (Armoury Crate) and lightweight controllers (G-Helper) restrict 4-zone models to static, breathing, or color-cycle modes.

**AuraZoneRipple** brings **real-time reactive ripple typing** to ASUS ROG 4-Zone laptops through a software-driven high-frequency (50 FPS) USB HID direct pipeline.

Unlike heavy RGB suites (SignalRGB, Armoury Crate) that consume hundreds of megabytes of RAM and multiple CPU cores, AuraZoneRipple is written in **native Rust**, consuming **less than 9 MB of RAM** and **under 0.2% CPU**.

---

## ✨ Features

- **🌊 4-Zone Keyboard + Front Wraparound Lightbar Waterfall**:
  - Keystrokes in any zone trigger smooth, cosine-eased wave ripples that propagate across the 4 keyboard zones and cascade downward onto the front underglow lightbar.
- **🖥️ Modern Cyberpunk Native GUI (`egui/eframe`)**:
  - Beautiful dark cyberpunk interface built with immediate-mode Rust GUI.
  - Live interactive keyboard deck & front lightbar visualizer connected in real time to physical keystrokes.
  - Real-time sliders for wave speed, decay duration, idle brightness dimming, and cascade delay.
  - 6 One-Click Presets: *Cyberpunk, Volcano, Matrix, Synthwave, Rainbow Keystrokes, and Stealth*.
- **🎮 Seamless G-Helper Integration**:
  - Syncs directly with G-Helper active profile colors, modes, and brightness settings without modifying or overriding your physical laptop macro keys.
  - Can be run as a standalone application, through desktop shortcuts, or as a background service.
- **⚡ Ultra-Low Resource Footprint**:
  - **~8.6 MB Working Set RAM** in background daemon mode.
  - **< 0.2% CPU utilization** using Windows asynchronous key state polling and low-level Win32 hooks.
  - Standalone release executable (~4.2 MB) with zero non-system DLL dependencies.
- **🔒 Single-Instance Win32 Mutex & Named Pipe IPC**:
  - Built-in IPC server (`\\.\pipe\AuraZoneRipple_IPC`) handling `--daemon`, `--toggle`, `--status`, `--stop`, and `--gui` commands without race conditions.
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

With this calibration, ripples travel in true 1:1 physical alignment across both the keyboard keys and the desk underglow.

---

## 🚀 Quick Start & Installation

### Option 1: Pre-Built Release Binary
1. Download or locate `aurazone_ripple.exe` in `target/release/`.
2. Double-click to run live in console, or launch the GUI:
   ```cmd
   aurazone_ripple.exe --gui
   ```

### Option 2: Build from Source
Ensure you have the Rust toolchain installed ([rustup.rs](https://rustup.rs)):

```bash
# Clone the repository
git clone https://github.com/smone-jovan/aurazone-ripple.git
cd aurazone-ripple

# Run test suite (133 tests)
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
AuraZoneRipple_Rust v0.1.0 — ASUS ROG Strix G513RC Aura Direct Lighting

USAGE:
    aurazone_ripple.exe [OPTIONS]

OPTIONS:
    --daemon              Run as background service daemon (Named Pipe IPC server)
    --gui                 Launch the modern cyberpunk Native GUI control center
    --toggle              Toggle ripple effect on/off (auto-spawns daemon if idle)
    --status              Query running daemon status and active preset
    --stop                Gracefully stop the background daemon and restore static lighting
    --restore             Restore default static RGB lighting profile from G-Helper
    --install-ghelper     Automatically configure G-Helper M5 action key to trigger --toggle
    --mock                Run in mock/emulation mode (no physical hardware required)
    -h, --help            Print help information
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
│   ├── gui/                 # Modern dark cyberpunk GUI (egui/eframe/glow)
│   ├── config.rs            # Preset management and atomic JSON configuration
│   ├── daemon.rs            # High-performance 50 FPS daemon orchestrator
│   ├── ghelper.rs           # G-Helper M5 binding installer & profile sync
│   └── main.rs              # CLI entry point router
├── tests/                   # Automated 4-tier verification test suite (133 tests)
├── python_prototype/        # Zero-dependency reference Python prototype & control panel
├── Cargo.toml               # Cargo package configuration
└── README.md                # Project documentation
```

---

## 🤝 Acknowledgements

- **[G-Helper](https://github.com/seerge/g-helper)** by `seerge` — The amazing lightweight control tool for ASUS laptops.
- **[OpenRGB](https://gitlab.com/CalcProgrammer1/OpenRGB)** — Hardware documentation for ASUS Aura USB endpoints.

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).
