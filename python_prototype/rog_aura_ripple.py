"""
================================================================================
 ASUS ROG Strix 4-Zone Reactive Ripple RGB Extension for G-Helper
 Target Hardware: ASUS ROG Strix G513RC (ASUS N-Key Device VID_0B05 & PID_19B6)
 Reverse Engineered via REA MCP from GHelper.dll (G-Helper v0.291)
 Pure Python 3 standard library + Win32 ctypes (Zero external dependencies)
================================================================================
"""

import os
import sys
import time
import math
import json
import winreg
import signal
import ctypes
from ctypes import wintypes
import threading
import subprocess

# Base Directory & Logging Setup
BASE_DIR = os.path.dirname(os.path.abspath(__file__))
LOG_FILE = os.path.join(BASE_DIR, "ripple.log")
PID_FILE = os.path.join(BASE_DIR, "ripple.pid")
CONFIG_FILE = os.path.join(BASE_DIR, "ripple_settings.json")
GHELPER_CONFIG_PATH = os.path.expandvars(r"%APPDATA%\GHelper\config.json")

# When launched via pythonw.exe, sys.stdout and sys.stderr are None
if sys.stdout is None:
    try:
        sys.stdout = open(LOG_FILE, "a", encoding="utf-8", buffering=1)
        sys.stderr = sys.stdout
    except Exception:
        pass
else:
    try:
        sys.stdout.reconfigure(line_buffering=True)
    except Exception:
        pass

import traceback
def global_excepthook(exc_type, exc_val, exc_tb):
    try:
        with open(LOG_FILE, "a", encoding="utf-8") as f:
            f.write(f"\n[CRASH PID {os.getpid()} - {time.strftime('%Y-%m-%d %H:%M:%S')}] Unhandled exception:\n")
            traceback.print_exception(exc_type, exc_val, exc_tb, file=f)
    except Exception:
        pass
sys.excepthook = global_excepthook

# Win32 API Library Handles
kernel32 = ctypes.windll.kernel32
user32 = ctypes.windll.user32
hid = ctypes.windll.hid

GENERIC_READ = 0x80000000
GENERIC_WRITE = 0x40000000
FILE_SHARE_READ = 1
FILE_SHARE_WRITE = 2
OPEN_EXISTING = 3

# Configure Win32 Function Prototypes for 64-bit safety
kernel32.CreateFileW.argtypes = [wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD, ctypes.c_void_p, wintypes.DWORD, wintypes.DWORD, wintypes.HANDLE]
kernel32.CreateFileW.restype = wintypes.HANDLE
kernel32.CloseHandle.argtypes = [wintypes.HANDLE]
kernel32.CloseHandle.restype = wintypes.BOOL
kernel32.GetLastError.restype = wintypes.DWORD

kernel32.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
kernel32.OpenProcess.restype = wintypes.HANDLE
kernel32.GetExitCodeProcess.argtypes = [wintypes.HANDLE, ctypes.POINTER(wintypes.DWORD)]
kernel32.GetExitCodeProcess.restype = wintypes.BOOL

user32.GetAsyncKeyState.restype = ctypes.c_short
user32.GetAsyncKeyState.argtypes = [ctypes.c_int]

# Aura Direct Protocol Constants (extracted from GHelper.USB.Aura)
AURA_ID = 0x5D
AURA_DIRECT = 0xBC

# G-Helper 4-Zone Hardware Slot Mapping for ROG Strix G513:
# 12 hardware LED slots mapped to 8 logical zones:
# Slots 0..5: Keyboard Zones (0=Left, 1=Mid-Left, 2=Mid-Right, 3=Right)
# Slots 6..11: Wraparound Lightbar Zones (Hardware bus wires ascending Right->Left: 7, 7, 6, 5, 4, 4)
ZONE_MAP_G513 = [0, 1, 2, 3, 0, 0, 7, 7, 6, 5, 4, 4]

DEFAULT_CONFIG = {
    "sync_with_ghelper": True,
    "dim_base_lighting": True,             # Dim idle lighting so reactive ripples pop vividly
    "base_dim_factor": 0.25,               # 25% brightness for idle glow
    "custom_base_color": [0, 0, 180],      # Fallback Base Color (RGB)
    "custom_ripple_color": [0, 240, 255],  # Fallback Vivid Neon Cyan Ripple
    "use_ghelper_color2_as_ripple": True,  # Use G-Helper Color 2 as Ripple Color
    "wave_speed": 0.065,                   # Seconds per zone step across keyboard
    "ripple_duration": 0.38,               # Full ripple envelope duration in seconds
    "lightbar_enabled": True,              # Enable waterfall cascade to front lightbar
    "lightbar_cascade_delay": 0.035,       # Delay before lightbar begins lighting
    "target_fps": 50                       # Frame rate for buttery smooth transitions
}


def load_settings():
    cfg = dict(DEFAULT_CONFIG)
    if os.path.exists(CONFIG_FILE):
        try:
            with open(CONFIG_FILE, "r", encoding="utf-8") as f:
                saved = json.load(f)
                cfg.update(saved)
        except Exception:
            pass
    else:
        try:
            with open(CONFIG_FILE, "w", encoding="utf-8") as f:
                json.dump(cfg, f, indent=2)
        except Exception:
            pass
    return cfg


def get_ghelper_profile():
    """Reads current lighting color and brightness config from G-Helper."""
    color1 = (0, 0, 255)       # Default Blue
    color2 = (255, 64, 0)      # Default Orange
    brightness = 3             # 0..3
    try:
        if os.path.exists(GHELPER_CONFIG_PATH):
            with open(GHELPER_CONFIG_PATH, "r", encoding="utf-8") as f:
                d = json.load(f)
            
            raw_c1 = d.get("aura_color", -16776961) & 0xFFFFFFFF
            color1 = ((raw_c1 >> 16) & 0xFF, (raw_c1 >> 8) & 0xFF, raw_c1 & 0xFF)
            
            raw_c2 = d.get("aura_color2", -49152) & 0xFFFFFFFF
            color2 = ((raw_c2 >> 16) & 0xFF, (raw_c2 >> 8) & 0xFF, raw_c2 & 0xFF)
            
            brightness = d.get("keyboard_brightness", 3)
    except Exception:
        pass
    return color1, color2, brightness


def find_col05_device_path():
    """
    Dynamically finds the Aura Col05 (Direct Mode) HID device interface via registry.
    Falls back to known ASUS Strix G513 hardware endpoint if needed.
    """
    base_key = r"SYSTEM\CurrentControlSet\Control\DeviceClasses\{4d1e55b2-f16f-11cf-88cb-001111000030}"
    try:
        with winreg.OpenKey(winreg.HKEY_LOCAL_MACHINE, base_key) as k:
            i = 0
            while True:
                try:
                    sub = winreg.EnumKey(k, i)
                    i += 1
                    if "0b05" in sub.lower() and "col05" in sub.lower():
                        return "\\\\?\\" + sub[4:]
                except OSError:
                    break
    except Exception:
        pass
    return r"\\?\HID#VID_0B05&PID_19B6&Col05#7&2d1c04e0&0&0004#{4d1e55b2-f16f-11cf-88cb-001111000030}"


class AuraDirectDevice:
    """Handles communication with the ASUS Aura HID Col05 Direct Lighting Controller."""
    def __init__(self):
        self.handle = None
        self.device_path = find_col05_device_path()
        self._open()

    def _open(self):
        self.handle = kernel32.CreateFileW(
            self.device_path, GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None, OPEN_EXISTING, 0, None
        )
        if self.handle == -1 or self.handle == 0:
            err = kernel32.GetLastError()
            self.handle = None
            raise RuntimeError(f"Gagal membuka endpoint Aura Col05 HID (Win32 Error: {err})")

        # Send G-Helper initialization primer handshake (64 bytes)
        init_pkt = bytearray(64)
        init_pkt[0] = AURA_ID      # 0x5D
        init_pkt[1] = AURA_DIRECT  # 0xBC
        init_pkt[2] = 0x01         # Handshake init byte
        self._send_feature(init_pkt)
        time.sleep(0.05)

    def _send_feature(self, buf):
        if not self.handle:
            return False
        c_buf = (ctypes.c_char * len(buf)).from_buffer_copy(buf)
        ok = bool(hid.HidD_SetFeature(self.handle, c_buf, len(buf)))
        if not ok:
            # Attempt auto-reconnect on communication loss
            try:
                kernel32.CloseHandle(self.handle)
            except Exception:
                pass
            self.handle = None
            try:
                self._open()
                return bool(hid.HidD_SetFeature(self.handle, c_buf, len(buf)))
            except Exception:
                return False
        return True

    def apply_zones(self, zone_colors):
        """
        zone_colors: list of 8 (R, G, B) tuples.
        0..3 = Keyboard Zones Left to Right
        4..7 = Lightbar Zones
        """
        pkt = bytearray(64)
        pkt[0] = AURA_ID
        pkt[1] = AURA_DIRECT
        pkt[2] = 0x00
        pkt[3] = 0x01
        pkt[4] = 0x04  # 4-Zone Command Code

        for i, z in enumerate(ZONE_MAP_G513):
            c = zone_colors[z]
            off = 9 + i * 3
            pkt[off]     = max(0, min(255, int(c[0])))
            pkt[off + 1] = max(0, min(255, int(c[1])))
            pkt[off + 2] = max(0, min(255, int(c[2])))

        return self._send_feature(pkt)

    def set_static_uniform(self, color):
        """Applies a uniform static color across all zones."""
        colors = [color] * 8
        return self.apply_zones(colors)

    def close(self):
        if self.handle:
            try:
                kernel32.CloseHandle(self.handle)
            except Exception:
                pass
            self.handle = None


# Virtual Key Code Mapping for ASUS Keyboard layout
# Maps physical keys directly into one of the 4 keyboard zones (0, 1, 2, 3)
VK_ZONE_MAP = {}

# Zone 0: Left Keyboard
# Esc, F1-F4, `~, 1-3, Tab, QWE, Caps, ASD, LShift, ZXC, LCtrl, LWin, LAlt, Top ROG M1-M2
for vk in [
    0x1B,                   # VK_ESCAPE
    0x70, 0x71, 0x72, 0x73, # F1, F2, F3, F4
    0xC0,                   # `~
    0x31, 0x32, 0x33,       # 1, 2, 3
    0x09,                   # TAB
    0x51, 0x57, 0x45,       # Q, W, E
    0x14,                   # CAPS LOCK
    0x41, 0x53, 0x44,       # A, S, D
    0xA0,                   # LSHIFT
    0x5A, 0x58, 0x43,       # Z, X, C
    0xA2,                   # LCONTROL
    0x5B,                   # LWIN
    0xA4,                   # LMENU / LALT
    0xAD, 0xAE,             # Volume Mute, Volume Down (M1, M2)
]:
    VK_ZONE_MAP[vk] = 0

# Zone 1: Mid-Left Keyboard
# F5-F6, 4-6, RTY, FGH, VB, Space, Top ROG M3
for vk in [
    0x74, 0x75,             # F5, F6
    0x34, 0x35, 0x36,       # 4, 5, 6
    0x52, 0x54, 0x59,       # R, T, Y
    0x46, 0x47, 0x48,       # F, G, H
    0x56, 0x42,             # V, B
    0x20,                   # SPACE
    0xAF,                   # Volume Up (M3)
]:
    VK_ZONE_MAP[vk] = 1

# Zone 2: Mid-Right Keyboard
# F7-F9, 7-9, UIO, JKL, NM, Comma, RAlt, Top ROG M4
for vk in [
    0x76, 0x77, 0x78,       # F7, F8, F9
    0x37, 0x38, 0x39,       # 7, 8, 9
    0x55, 0x49, 0x4F,       # U, I, O
    0x4A, 0x4B, 0x4C,       # J, K, L
    0x4E, 0x4D,             # N, M
    0xBC,                   # ,< (comma)
    0xA5,                   # RMENU / RALT
]:
    VK_ZONE_MAP[vk] = 2

# Zone 3: Right Keyboard & Numpad & Navigation & Arrows
# F10-F12, 0, -, =, Bksp, P, [], \, Enter, quotes, dot, slash, Arrows, Numpad, Top ROG M5
for vk in [
    0x79, 0x7A, 0x7B,       # F10, F11, F12
    0x30, 0xBD, 0xBB,       # 0, -, =
    0x08,                   # BACKSPACE
    0x50, 0xDB, 0xDD, 0xDC, # P, [, ], \
    0xBA, 0xDE,             # ;:, '"
    0x0D,                   # RETURN / ENTER
    0xBE, 0xBF,             # .>, /?
    0xA1, 0xA3,             # RSHIFT, RCONTROL
    0x25, 0x26, 0x27, 0x28, # LEFT, UP, RIGHT, DOWN
    0x2D, 0x2E, 0x24, 0x23, 0x21, 0x22, # INS, DEL, HOME, END, PGUP, PGDN
]:
    VK_ZONE_MAP[vk] = 3

# Numpad (0x60..0x6F) -> Zone 3
for vk in range(0x60, 0x70):
    VK_ZONE_MAP[vk] = 3

# Generic fallback modifier keys
VK_ZONE_MAP[0x10] = 0 # VK_SHIFT
VK_ZONE_MAP[0x11] = 0 # VK_CONTROL
VK_ZONE_MAP[0x12] = 0 # VK_MENU


class ReactiveRippleEngine:
    """Calculates fluid wave physics and renders reactive ripple lighting to Aura HID."""
    def __init__(self, aura_dev, settings):
        self.aura = aura_dev
        self.settings = settings
        self.running = True
        self.lock = threading.Lock()
        self.active_ripples = []
        self.last_vk_times = {}     # Per-key debounce table
        self.last_zone_times = {}   # Per-zone debounce table

        # Dynamic G-Helper synced colors
        self.base_color = (0, 0, 180)
        self.ripple_color = (0, 240, 255)
        self.brightness_scale = 1.0

        self.last_cfg_check = 0.0
        self._refresh_colors()

    def _refresh_colors(self):
        if self.settings.get("sync_with_ghelper", True):
            c1, c2, bright = get_ghelper_profile()
            scales = [0.0, 0.33, 0.66, 1.0]
            self.brightness_scale = scales[min(3, max(0, bright))]

            # Compute Base Color
            if self.settings.get("dim_base_lighting", True):
                factor = float(self.settings.get("base_dim_factor", 0.25))
                self.base_color = (
                    int(c1[0] * factor * self.brightness_scale),
                    int(c1[1] * factor * self.brightness_scale),
                    int(c1[2] * factor * self.brightness_scale)
                )
            else:
                self.base_color = (
                    int(c1[0] * self.brightness_scale),
                    int(c1[1] * self.brightness_scale),
                    int(c1[2] * self.brightness_scale)
                )

            # Compute Ripple Color
            if self.settings.get("use_ghelper_color2_as_ripple", True) and c2 != c1:
                self.ripple_color = (
                    int(c2[0] * self.brightness_scale),
                    int(c2[1] * self.brightness_scale),
                    int(c2[2] * self.brightness_scale)
                )
            else:
                rc = self.settings.get("custom_ripple_color", [0, 240, 255])
                self.ripple_color = (
                    int(rc[0] * self.brightness_scale),
                    int(rc[1] * self.brightness_scale),
                    int(rc[2] * self.brightness_scale)
                )
        else:
            bc = self.settings.get("custom_base_color", [0, 0, 180])
            rc = self.settings.get("custom_ripple_color", [0, 240, 255])
            self.brightness_scale = 1.0

            # Compute Base Color with idle dimming factor
            if self.settings.get("dim_base_lighting", True):
                factor = float(self.settings.get("base_dim_factor", 0.25))
                self.base_color = (
                    int(bc[0] * factor),
                    int(bc[1] * factor),
                    int(bc[2] * factor)
                )
            else:
                self.base_color = tuple(bc)

            # Compute Ripple Color
            self.ripple_color = tuple(rc)

    def trigger_keystroke(self, zone, vk=None):
        now = time.time()
        # Debounce per virtual key (60ms) to prevent key repeat flooding
        if vk is not None:
            last_t = self.last_vk_times.get(vk, 0.0)
            if now - last_t < 0.060:
                return
            self.last_vk_times[vk] = now
        else:
            last_t = self.last_zone_times.get(zone, 0.0)
            if now - last_t < 0.040:
                return
            self.last_zone_times[zone] = now

        zone_names = ["Zona 0 (Kiri/WASD)", "Zona 1 (Tengah-Kiri)", "Zona 2 (Tengah-Kanan)", "Zona 3 (Kanan/Numpad)"]
        try:
            vk_info = f" (VK 0x{vk:02X})" if vk else ""
            print(f" -> [KETIK] Terdeteksi di {zone_names[zone]}{vk_info}! Gelombang merambat...", flush=True)
        except Exception:
            pass

        with self.lock:
            # Maintain up to 8 concurrent ripples for rich ripple overlapping
            if len(self.active_ripples) >= 8:
                self.active_ripples.pop(0)

            if self.settings.get("rainbow_mode", False):
                import colorsys
                self.rainbow_step = (getattr(self, 'rainbow_step', 0.0) + 0.12) % 1.0
                rgb = colorsys.hsv_to_rgb(self.rainbow_step, 1.0, 1.0)
                rip_c = (int(rgb[0] * 255 * self.brightness_scale), int(rgb[1] * 255 * self.brightness_scale), int(rgb[2] * 255 * self.brightness_scale))
            else:
                rip_c = self.ripple_color

            self.active_ripples.append({
                'origin': zone,
                't0': now,
                'color': rip_c
            })

    def run(self):
        fps = float(self.settings.get("target_fps", 50))
        frame_duration = 1.0 / fps
        wave_spd = float(self.settings.get("wave_speed", 0.065))
        rip_dur = float(self.settings.get("ripple_duration", 0.38))
        lightbar_on = bool(self.settings.get("lightbar_enabled", True))
        bar_delay = float(self.settings.get("lightbar_cascade_delay", 0.035))

        while self.running:
            start_t = time.time()

            # Refresh G-Helper config and ripple settings every 1.5 seconds
            if start_t - self.last_cfg_check > 1.5:
                self.settings = load_settings()
                wave_spd = float(self.settings.get("wave_speed", 0.065))
                rip_dur = float(self.settings.get("ripple_duration", 0.38))
                lightbar_on = bool(self.settings.get("lightbar_enabled", True))
                bar_delay = float(self.settings.get("lightbar_cascade_delay", 0.035))
                self._refresh_colors()
                self.last_cfg_check = start_t

            # If brightness is 0 (off in G-Helper), turn off all LEDs
            if self.brightness_scale <= 0.0:
                try:
                    self.aura.set_static_uniform((0, 0, 0))
                except Exception:
                    pass
                time.sleep(0.1)
                continue

            # Calculate intensities for 8 zones (0..3 keyboard, 4..7 lightbar)
            zone_intensities = [0.0] * 8
            zone_color_sums = [[0.0, 0.0, 0.0] for _ in range(8)]

            with self.lock:
                surviving = []
                for rip in self.active_ripples:
                    origin = rip['origin']
                    age = start_t - rip['t0']
                    rip_c = rip.get('color', self.ripple_color)
                    is_alive = False

                    # 1. Keyboard zones 0..3
                    for z in range(4):
                        dist = abs(z - origin)
                        arr_time = dist * wave_spd
                        rel_t = age - arr_time
                        if 0.0 <= rel_t <= rip_dur:
                            is_alive = True
                            norm_t = rel_t / rip_dur
                            # Smooth attack (15% rise) and cosine-squared decay (85% ease-out)
                            if norm_t < 0.15:
                                intensity = math.sin((norm_t / 0.15) * (math.pi / 2))
                            else:
                                decay_frac = (norm_t - 0.15) / 0.85
                                intensity = math.cos(decay_frac * (math.pi / 2)) ** 2
                            
                            dist_attenuation = 1.0 - (dist * 0.08)
                            net_i = intensity * dist_attenuation
                            zone_intensities[z] = min(1.0, zone_intensities[z] + net_i)
                            for ci in range(3):
                                zone_color_sums[z][ci] += rip_c[ci] * net_i

                    # 2. Lightbar zones 4..7 (cascades downward from keyboard)
                    if lightbar_on:
                        for lz in range(4):
                            kb_equiv = lz # 0->4, 1->5, 2->6, 3->7
                            dist = abs(kb_equiv - origin)
                            arr_time = (dist * wave_spd) + bar_delay
                            rel_t = age - arr_time
                            if 0.0 <= rel_t <= rip_dur:
                                is_alive = True
                                norm_t = rel_t / rip_dur
                                if norm_t < 0.15:
                                    intensity = math.sin((norm_t / 0.15) * (math.pi / 2))
                                else:
                                    decay_frac = (norm_t - 0.15) / 0.85
                                    intensity = math.cos(decay_frac * (math.pi / 2)) ** 2
                                dist_attenuation = 1.0 - (dist * 0.08)
                                net_i = intensity * 0.95 * dist_attenuation
                                zone_intensities[4 + lz] = min(1.0, zone_intensities[4 + lz] + net_i)
                                for ci in range(3):
                                    zone_color_sums[4 + lz][ci] += rip_c[ci] * net_i

                    if is_alive or age < (3 * wave_spd + rip_dur + bar_delay):
                        surviving.append(rip)

                self.active_ripples = surviving

            # Blend Base Color with Ripple Color based on calculated zone intensity
            output_colors = []
            for z in range(8):
                factor = zone_intensities[z]
                if factor > 0:
                    r_c = [min(255, int(zone_color_sums[z][i] / factor)) for i in range(3)]
                else:
                    r_c = self.ripple_color
                r = int(self.base_color[0] + (r_c[0] - self.base_color[0]) * factor)
                g = int(self.base_color[1] + (r_c[1] - self.base_color[1]) * factor)
                b = int(self.base_color[2] + (r_c[2] - self.base_color[2]) * factor)
                output_colors.append((r, g, b))

            try:
                self.aura.apply_zones(output_colors)
            except Exception:
                pass

            elapsed = time.time() - start_t
            sleep_t = max(0.001, frame_duration - elapsed)
            time.sleep(sleep_t)


# Win32 Low-Level Keyboard Hook Structures & Global Anchor
class KBDLLHOOKSTRUCT(ctypes.Structure):
    _fields_ = [
        ("vkCode", wintypes.DWORD),
        ("scanCode", wintypes.DWORD),
        ("flags", wintypes.DWORD),
        ("time", wintypes.DWORD),
        ("dwExtraInfo", ctypes.c_size_t),
    ]

HOOKPROC = ctypes.WINFUNCTYPE(ctypes.c_ssize_t, ctypes.c_int, wintypes.WPARAM, wintypes.LPARAM)
user32.SetWindowsHookExW.argtypes = [ctypes.c_int, HOOKPROC, wintypes.HINSTANCE, wintypes.DWORD]
user32.SetWindowsHookExW.restype = wintypes.HHOOK
user32.UnhookWindowsHookEx.argtypes = [wintypes.HHOOK]
user32.UnhookWindowsHookEx.restype = wintypes.BOOL
user32.CallNextHookEx.argtypes = [wintypes.HHOOK, ctypes.c_int, wintypes.WPARAM, wintypes.LPARAM]
user32.CallNextHookEx.restype = ctypes.c_ssize_t

_GLOBAL_HOOK_PROC = None

def start_keystroke_hook(engine):
    """
    Win32 Low-Level Keyboard Hook (WH_KEYBOARD_LL) in a dedicated message loop thread.
    Zero CPU polling overhead and instantaneous keystroke detection.
    """
    global _GLOBAL_HOOK_PROC
    WH_KEYBOARD_LL = 13
    WM_KEYDOWN = 0x0100
    WM_SYSKEYDOWN = 0x0104

    hook_h = None

    def hook_proc(nCode, wParam, lParam):
        try:
            if nCode >= 0 and lParam and wParam in (WM_KEYDOWN, WM_SYSKEYDOWN):
                kb = KBDLLHOOKSTRUCT.from_address(lParam)
                vk = kb.vkCode
                zone = VK_ZONE_MAP.get(vk, 1)
                engine.trigger_keystroke(zone, vk=vk)
        except Exception:
            pass
        return user32.CallNextHookEx(hook_h, nCode, wParam, lParam)

    _GLOBAL_HOOK_PROC = HOOKPROC(hook_proc)

    def hook_thread_loop():
        nonlocal hook_h
        try:
            hook_h = user32.SetWindowsHookExW(WH_KEYBOARD_LL, _GLOBAL_HOOK_PROC, None, 0)
            msg = wintypes.MSG()
            while engine.running and user32.GetMessageW(ctypes.byref(msg), 0, 0, 0) > 0:
                user32.TranslateMessage(ctypes.byref(msg))
                user32.DispatchMessageW(ctypes.byref(msg))
        except Exception:
            pass
        finally:
            if hook_h:
                try:
                    user32.UnhookWindowsHookEx(hook_h)
                except Exception:
                    pass

    t = threading.Thread(target=hook_thread_loop, daemon=True, name="WH_KEYBOARD_LL")
    t.start()
    return t


def start_asynckeystate_poller(engine):
    """
    Complementary polling listener using GetAsyncKeyState.
    Guarantees keystrokes are caught even when UIPI or elevated windows block hooks.
    """
    # Unique physical keys to poll (excluding generic fallbacks)
    unique_keys = [(vk, zone) for vk, zone in VK_ZONE_MAP.items() if vk not in (0x10, 0x11, 0x12)]

    def poller_loop():
        last_states = {}
        while engine.running:
            for vk, zone in unique_keys:
                state = user32.GetAsyncKeyState(vk)
                is_down = bool(state & 0x8000)
                was_down = last_states.get(vk, False)
                if is_down and not was_down:
                    engine.trigger_keystroke(zone, vk=vk)
                last_states[vk] = is_down
            time.sleep(0.012) # ~83 Hz polling rate (<0.1% CPU)

    t = threading.Thread(target=poller_loop, daemon=True, name="GetAsyncKeyStatePoller")
    t.start()
    return t


def is_pid_running(pid):
    """Checks if a Windows process is currently alive."""
    # PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
    h = kernel32.OpenProcess(0x1000, False, pid)
    if h:
        exit_code = wintypes.DWORD()
        res = kernel32.GetExitCodeProcess(h, ctypes.byref(exit_code))
        kernel32.CloseHandle(h)
        # STILL_ACTIVE = 259 (0x103)
        return bool(res and exit_code.value == 259)
    return False


def get_running_pid():
    """Returns the running PID if active, otherwise cleans stale PID file."""
    if os.path.exists(PID_FILE):
        try:
            with open(PID_FILE, "r") as f:
                pid = int(f.read().strip())
            if is_pid_running(pid):
                return pid
            else:
                try:
                    os.remove(PID_FILE)
                except Exception:
                    pass
        except Exception:
            pass
    return None


def restore_ghelper_static():
    """Restores full static G-Helper lighting and brightness on exit."""
    try:
        dev = AuraDirectDevice()
        c1, _, bright = get_ghelper_profile()
        scales = [0.0, 0.33, 0.66, 1.0]
        s = scales[min(3, max(0, bright))]
        static_c = (int(c1[0] * s), int(c1[1] * s), int(c1[2] * s))
        dev.set_static_uniform(static_c)
        dev.close()
        print(f"[RESTORE] Static lighting restored to RGB{static_c}.")
        return True
    except Exception as e:
        print(f"[ERROR] Restore failed: {e}")
        return False


def stop_running_daemon():
    """Stops any existing running Aura Ripple process via PID file."""
    running_pid = get_running_pid()
    if running_pid:
        print(f"Menghentikan proses Aura Ripple (PID {running_pid})...")
        os.system(f"taskkill /F /PID {running_pid} >nul 2>&1")
        # Wait up to 1.5 seconds for clean termination
        for _ in range(15):
            if not is_pid_running(running_pid):
                break
            time.sleep(0.1)
        if os.path.exists(PID_FILE):
            try:
                os.remove(PID_FILE)
            except Exception:
                pass
    restore_ghelper_static()


def start_background_daemon():
    """Launches the engine invisibly via pythonw.exe in background daemon mode."""
    running_pid = get_running_pid()
    if running_pid:
        print(f"[INFO] Aura Ripple sudah aktif di latar belakang (PID {running_pid}).")
        return True

    # Clean any stale PID file
    if os.path.exists(PID_FILE):
        try:
            os.remove(PID_FILE)
        except Exception:
            pass

    pythonw = os.path.join(os.path.dirname(sys.executable), "pythonw.exe")
    if not os.path.exists(pythonw):
        pythonw = sys.executable

    script_path = os.path.abspath(__file__)
    # For pythonw GUI binary, CREATE_NEW_PROCESS_GROUP launches cleanly detached
    proc = subprocess.Popen(
        [pythonw, script_path, "--daemon"],
        creationflags=subprocess.CREATE_NEW_PROCESS_GROUP,
        close_fds=True
    )

    # Wait up to 2 seconds for worker to write PID
    for _ in range(20):
        time.sleep(0.1)
        r_pid = get_running_pid()
        if r_pid and is_pid_running(r_pid):
            print(f"[SUKSES] Aura Ripple berhasil dijalankan di latar belakang (PID {r_pid}).")
            return True

    if is_pid_running(proc.pid):
        print(f"[SUKSES] Aura Ripple berhasil dijalankan di latar belakang (PID {proc.pid}).")
        return True

    print("[PERINGATAN] Proses telah diluncurkan namun belum terdeteksi di status.")
    return False


def toggle_daemon():
    """Toggles the background daemon ON/OFF."""
    running_pid = get_running_pid()
    if running_pid:
        print(f"Menghentikan Aura Ripple (PID {running_pid})...")
        stop_running_daemon()
        print("[STATUS] Aura Ripple: NONAKTIF (Pencahayaan statis G-Helper aktif).")
    else:
        print("Menyalakan Aura Ripple di latar belakang...")
        start_background_daemon()
        print("[STATUS] Aura Ripple: AKTIF (Efek riak reaktif menyala).")


def run_engine_worker():
    """Core worker loop executed in daemon or interactive mode."""
    # Record PID
    with open(PID_FILE, "w") as f:
        f.write(str(os.getpid()))

    settings = load_settings()

    print("=" * 65)
    print(" ASUS ROG Strix 4-Zone Reactive Ripple Engine (G-Helper Ext.)")
    print(" Target: ASUS ROG Strix G513RC (ASUS N-Key VID_0B05 PID_19B6)")
    print("=" * 65)
    print("Membuka kontroler Aura Direct HID (Col05)...")

    try:
        aura = AuraDirectDevice()
        print(f"[OK] Kontroler Aura Direct aktif pada:\n     {aura.device_path}")
    except Exception as e:
        print(f"[FATAL] Gagal menginisialisasi Aura HID: {e}")
        if os.path.exists(PID_FILE):
            try:
                os.remove(PID_FILE)
            except Exception:
                pass
        sys.exit(1)

    c1, c2, b = get_ghelper_profile()
    print(f"[G-Helper Sync] Base: RGB{c1} | Ripple: RGB{c2} | Brightness: {b}/3")

    # Visual handshake: momentarily flash ripple color to signal activation
    try:
        aura.set_static_uniform((0, 240, 255))
        time.sleep(0.12)
    except Exception:
        pass

    engine = ReactiveRippleEngine(aura, settings)

    def exit_gracefully(sig=None, frame=None):
        print("\nMenerima sinyal berhenti, memulihkan pencahayaan...")
        engine.running = False
        time.sleep(0.08)
        try:
            restore_ghelper_static()
        except Exception:
            pass
        if os.path.exists(PID_FILE):
            try:
                os.remove(PID_FILE)
            except Exception:
                pass
        sys.exit(0)

    try:
        signal.signal(signal.SIGINT, exit_gracefully)
        signal.signal(signal.SIGTERM, exit_gracefully)
    except Exception:
        pass

    # Start dual keystroke capture engines: Hook (zero latency) + Poller (UIPI backup)
    try:
        start_keystroke_hook(engine)
    except Exception:
        pass
    start_asynckeystate_poller(engine)

    # Start rendering engine thread
    render_thread = threading.Thread(target=engine.run, daemon=True, name="RenderEngine")
    render_thread.start()

    print("\n[SUKSES] Efek Ripple 4-Zone + Lightbar berhasil diaktifkan!")
    print("-> Silakan ketik apa saja di keyboard Anda untuk melihat efek riak.")
    print("-> Tekan Ctrl + C untuk keluar dan mengembalikan lampu ke G-Helper.\n")

    try:
        while engine.running:
            time.sleep(0.5)
    except (KeyboardInterrupt, SystemExit):
        exit_gracefully()
    except Exception as e:
        with open(LOG_FILE, "a", encoding="utf-8") as f:
            f.write(f"[ERROR PID {os.getpid()}] Exception in main loop: {e}\n")
    finally:
        exit_gracefully()


def main():
    if "--restore" in sys.argv:
        restore_ghelper_static()
        sys.exit(0)

    if "--stop" in sys.argv:
        stop_running_daemon()
        sys.exit(0)

    if "--toggle" in sys.argv:
        toggle_daemon()
        sys.exit(0)

    if "--start-bg" in sys.argv:
        start_background_daemon()
        sys.exit(0)

    if "--status" in sys.argv:
        running_pid = get_running_pid()
        if running_pid:
            print(f"RUNNING (PID {running_pid})")
            sys.exit(0)
        else:
            print("STOPPED")
            sys.exit(1)

    if "--daemon" in sys.argv:
        # Worker daemon mode
        run_engine_worker()
        sys.exit(0)

    # Interactive mode (stop previous daemon first)
    stop_running_daemon()
    run_engine_worker()


if __name__ == "__main__":
    main()
