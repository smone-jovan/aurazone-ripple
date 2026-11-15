"""
ASUS ROG Strix 4-Zone Reactive Ripple - Control Panel (GUI)
Modern Dark Theme Control Panel with Real-time Interactive Keyboard & Lightbar Visualizer.
Zero external dependencies (uses native Python tkinter + ttk + Win32 ctypes).
"""

import os
import sys
import time
import math
import json
import tkinter as tk
from tkinter import ttk, colorchooser, messagebox
import subprocess
import threading
import ctypes
from ctypes import wintypes

BASE_DIR = os.path.dirname(os.path.abspath(__file__))
CONFIG_FILE = os.path.join(BASE_DIR, "ripple_settings.json")
PID_FILE = os.path.join(BASE_DIR, "ripple.pid")
ENGINE_SCRIPT = os.path.join(BASE_DIR, "rog_aura_ripple.py")

user32 = ctypes.windll.user32

DEFAULT_SETTINGS = {
    "sync_with_ghelper": False,
    "dim_base_lighting": True,
    "base_dim_factor": 0.25,
    "custom_base_color": [10, 0, 35],
    "custom_ripple_color": [0, 240, 255],
    "use_ghelper_color2_as_ripple": False,
    "wave_speed": 0.065,
    "ripple_duration": 0.38,
    "lightbar_enabled": True,
    "lightbar_cascade_delay": 0.035,
    "target_fps": 50,
    "rainbow_mode": False
}

PRESETS = {
    "Cyberpunk": {
        "sync_with_ghelper": False,
        "custom_base_color": [20, 0, 45],
        "custom_ripple_color": [0, 245, 255],
        "rainbow_mode": False,
        "base_dim_factor": 0.25,
        "wave_speed": 0.065,
        "ripple_duration": 0.40,
        "use_ghelper_color2_as_ripple": False
    },
    "Volcano": {
        "sync_with_ghelper": False,
        "custom_base_color": [40, 0, 0],
        "custom_ripple_color": [255, 120, 0],
        "rainbow_mode": False,
        "base_dim_factor": 0.25,
        "wave_speed": 0.060,
        "ripple_duration": 0.42,
        "use_ghelper_color2_as_ripple": False
    },
    "Matrix": {
        "sync_with_ghelper": False,
        "custom_base_color": [0, 30, 5],
        "custom_ripple_color": [0, 255, 70],
        "rainbow_mode": False,
        "base_dim_factor": 0.25,
        "wave_speed": 0.055,
        "ripple_duration": 0.35,
        "use_ghelper_color2_as_ripple": False
    },
    "Synthwave": {
        "sync_with_ghelper": False,
        "custom_base_color": [5, 10, 40],
        "custom_ripple_color": [255, 0, 128],
        "rainbow_mode": False,
        "base_dim_factor": 0.25,
        "wave_speed": 0.065,
        "ripple_duration": 0.45,
        "use_ghelper_color2_as_ripple": False
    },
    "Rainbow Keystrokes": {
        "sync_with_ghelper": False,
        "custom_base_color": [10, 10, 20],
        "custom_ripple_color": [255, 255, 255],
        "rainbow_mode": True,
        "base_dim_factor": 0.20,
        "wave_speed": 0.065,
        "ripple_duration": 0.40,
        "use_ghelper_color2_as_ripple": False
    },
    "Stealth (Mati saat Diam)": {
        "sync_with_ghelper": False,
        "custom_base_color": [0, 0, 0],
        "custom_ripple_color": [0, 230, 255],
        "rainbow_mode": False,
        "base_dim_factor": 0.0,
        "wave_speed": 0.065,
        "ripple_duration": 0.45,
        "use_ghelper_color2_as_ripple": False
    }
}

VK_ZONE_MAP = {}
for vk in [0x1B, 0x70, 0x71, 0x72, 0x73, 0xC0, 0x31, 0x32, 0x33, 0x09, 0x51, 0x57, 0x45, 0x14, 0x41, 0x53, 0x44, 0x10, 0x5A, 0x58, 0x43, 0x11, 0x5B, 0x12]:
    VK_ZONE_MAP[vk] = 0
for vk in [0x74, 0x75, 0x34, 0x35, 0x36, 0x52, 0x54, 0x59, 0x46, 0x47, 0x48, 0x56, 0x42, 0x20]:
    VK_ZONE_MAP[vk] = 1
for vk in [0x76, 0x77, 0x78, 0x37, 0x38, 0x39, 0x55, 0x49, 0x4F, 0x4A, 0x4B, 0x4C, 0x4E, 0x4D, 0xBC, 0xA5]:
    VK_ZONE_MAP[vk] = 2
for vk in [0x79, 0x7A, 0x7B, 0x30, 0xBD, 0xBB, 0x08, 0x50, 0xDB, 0xDD, 0xDC, 0xBA, 0xDE, 0x0D, 0xBE, 0xBF, 0xA1, 0xA3, 0x25, 0x26, 0x27, 0x28, 0x2D, 0x2E, 0x24, 0x23, 0x21, 0x22]:
    VK_ZONE_MAP[vk] = 3
for vk in range(0x60, 0x70):
    VK_ZONE_MAP[vk] = 3


def load_config():
    cfg = dict(DEFAULT_SETTINGS)
    if os.path.exists(CONFIG_FILE):
        try:
            with open(CONFIG_FILE, "r", encoding="utf-8") as f:
                cfg.update(json.load(f))
        except Exception:
            pass
    return cfg


def save_config(cfg):
    try:
        with open(CONFIG_FILE, "w", encoding="utf-8") as f:
            json.dump(cfg, f, indent=2)
    except Exception as e:
        messagebox.showerror("Gagal Menyimpan", f"Error menyimpan config: {e}")


def is_running():
    try:
        res = subprocess.run([sys.executable, ENGINE_SCRIPT, "--status"], capture_output=True, text=True)
        return "RUNNING" in res.stdout
    except Exception:
        return False


class RippleControlPanel(tk.Tk):
    def __init__(self):
        super().__init__()
        self.title("ASUS ROG Strix - Aura Ripple Control Center")
        self.geometry("560x780")
        self.resizable(False, False)
        self.configure(bg="#101014")

        self.cfg = load_config()

        # Minimalist Dark Palette
        self.c_bg = "#101014"
        self.c_card = "#181822"
        self.c_card_border = "#262636"
        self.c_text = "#e6e6f0"
        self.c_muted = "#7a7a92"
        self.c_accent = "#00d2ff"
        self.c_red = "#ff4466"
        self.c_green = "#00e676"

        # Simulation & Visualizer State
        self.active_ripples = []
        self.ripple_lock = threading.Lock()
        self.rainbow_step = 0.0
        self.running_listener = True

        self._init_ui()
        self._start_key_listener()
        self._animate_visualizer()
        self._refresh_status()

    def _init_ui(self):
        # Header
        hdr = tk.Frame(self, bg=self.c_bg, padx=20, pady=10)
        hdr.pack(fill="x")

        lbl_title = tk.Label(hdr, text="AURA ZONE RIPPLE", font=("Segoe UI", 15, "bold"), fg=self.c_accent, bg=self.c_bg)
        lbl_title.pack(anchor="w")

        lbl_sub = tk.Label(hdr, text="ASUS ROG Strix G513RC • Stealth Edition", font=("Segoe UI", 9), fg=self.c_muted, bg=self.c_bg)
        lbl_sub.pack(anchor="w")

        # Status & Toggle Header Card
        card_status = tk.Frame(self, bg=self.c_card, padx=16, pady=8, highlightthickness=1, highlightbackground=self.c_card_border)
        card_status.pack(fill="x", padx=16, pady=4)

        self.lbl_status = tk.Label(card_status, text="Status: Memeriksa...", font=("Segoe UI", 9, "bold"), fg=self.c_text, bg=self.c_card)
        self.lbl_status.pack(side="left")

        btn_box = tk.Frame(card_status, bg=self.c_card)
        btn_box.pack(side="right")

        self.btn_toggle = tk.Button(btn_box, text="⚡ TOGGLE (ON/OFF)", font=("Segoe UI", 8, "bold"), bg="#222234", fg=self.c_accent, activebackground="#30304a", activeforeground=self.c_accent, bd=0, padx=10, pady=4, cursor="hand2", command=self._on_toggle)
        self.btn_toggle.pack(side="left", padx=4)

        # Interactive Live Visualizer Deck (4-Zone Keyboard + Front Lightbar)
        card_vis = tk.LabelFrame(self, text="  🖥️ LIVE KEYBOARD & LIGHTBAR DECK (TERHUBUNG)  ", font=("Segoe UI", 8, "bold"), fg=self.c_accent, bg=self.c_card, padx=12, pady=6, highlightthickness=1, highlightbackground=self.c_card_border)
        card_vis.pack(fill="x", padx=16, pady=4)

        self.canvas_w = 490
        self.canvas_h = 95
        self.canvas = tk.Canvas(card_vis, width=self.canvas_w, height=self.canvas_h, bg="#0d0d12", highlightthickness=1, highlightbackground="#222230")
        self.canvas.pack(fill="x", pady=4)
        self.canvas.bind("<Button-1>", self._on_canvas_click)

        lbl_vis_hint = tk.Label(card_vis, text="💡 Coba ketik tuts di keyboard fisik Anda! Visualizer ini tersambung real-time dan akan berpendar.", font=("Segoe UI", 8, "italic"), fg=self.c_muted, bg=self.c_card)
        lbl_vis_hint.pack(anchor="w", pady=2)

        # Presets Card
        card_preset = tk.LabelFrame(self, text="  🎨 PRESET CEPAT 1-KLIK  ", font=("Segoe UI", 8, "bold"), fg=self.c_accent, bg=self.c_card, padx=12, pady=6, highlightthickness=1, highlightbackground=self.c_card_border)
        card_preset.pack(fill="x", padx=16, pady=4)

        p_row1 = tk.Frame(card_preset, bg=self.c_card)
        p_row1.pack(fill="x", pady=2)
        p_row2 = tk.Frame(card_preset, bg=self.c_card)
        p_row2.pack(fill="x", pady=2)

        p_list = list(PRESETS.keys())
        for name in p_list[:3]:
            b = tk.Button(p_row1, text=name, font=("Segoe UI", 8, "bold"), bg="#222232", fg=self.c_text, activebackground="#35354d", activeforeground="#ffffff", bd=0, padx=8, pady=4, cursor="hand2", command=lambda n=name: self._apply_preset(n))
            b.pack(side="left", expand=True, fill="x", padx=3)

        for name in p_list[3:]:
            b = tk.Button(p_row2, text=name, font=("Segoe UI", 8, "bold"), bg="#222232", fg=self.c_text, activebackground="#35354d", activeforeground="#ffffff", bd=0, padx=8, pady=4, cursor="hand2", command=lambda n=name: self._apply_preset(n))
            b.pack(side="left", expand=True, fill="x", padx=3)

        # Color Settings Card
        card_color = tk.LabelFrame(self, text="  🌈 WARNA & KUSTOMISASI  ", font=("Segoe UI", 8, "bold"), fg=self.c_accent, bg=self.c_card, padx=12, pady=6, highlightthickness=1, highlightbackground=self.c_card_border)
        card_color.pack(fill="x", padx=16, pady=4)

        top_color_row = tk.Frame(card_color, bg=self.c_card)
        top_color_row.pack(fill="x", pady=2)

        self.var_sync = tk.BooleanVar(value=self.cfg.get("sync_with_ghelper", False))
        chk_sync = tk.Checkbutton(top_color_row, text="Ikuti Warna dari G-Helper (Auto-Sync)", variable=self.var_sync, font=("Segoe UI", 9, "bold"), fg=self.c_text, bg=self.c_card, selectcolor="#20202c", activebackground=self.c_card, activeforeground=self.c_text, command=self._on_sync_change)
        chk_sync.pack(side="left", anchor="w")

        self.var_rainbow = tk.BooleanVar(value=self.cfg.get("rainbow_mode", False))
        chk_rainbow = tk.Checkbutton(top_color_row, text="🌈 Mode Pelangi (Rainbow Keystrokes)", variable=self.var_rainbow, font=("Segoe UI", 9, "bold"), fg=self.c_accent, bg=self.c_card, selectcolor="#20202c", activebackground=self.c_card, activeforeground=self.c_accent, command=self._on_param_change)
        chk_rainbow.pack(side="right", anchor="e")

        col_box = tk.Frame(card_color, bg=self.c_card)
        col_box.pack(fill="x", pady=4)

        # Base color picker
        f_c1 = tk.Frame(col_box, bg=self.c_card)
        f_c1.pack(side="left", expand=True, fill="x", padx=4)
        tk.Label(f_c1, text="Warna Dasar (Idle):", font=("Segoe UI", 8), fg=self.c_muted, bg=self.c_card).pack(anchor="w")
        self.btn_c1 = tk.Button(f_c1, text="  Pilih Warna Dasar  ", font=("Segoe UI", 8, "bold"), bg=self._rgb_to_hex(self.cfg.get("custom_base_color", [10, 0, 35])), fg="#ffffff", bd=1, relief="ridge", cursor="hand2", command=self._pick_base_color)
        self.btn_c1.pack(fill="x", pady=2)

        # Ripple color picker
        f_c2 = tk.Frame(col_box, bg=self.c_card)
        f_c2.pack(side="left", expand=True, fill="x", padx=4)
        tk.Label(f_c2, text="Warna Riak (Ketikan):", font=("Segoe UI", 8), fg=self.c_muted, bg=self.c_card).pack(anchor="w")
        self.btn_c2 = tk.Button(f_c2, text="  Pilih Warna Riak  ", font=("Segoe UI", 8, "bold"), bg=self._rgb_to_hex(self.cfg.get("custom_ripple_color", [0, 240, 255])), fg="#000000", bd=1, relief="ridge", cursor="hand2", command=self._pick_ripple_color)
        self.btn_c2.pack(fill="x", pady=2)

        # Physics & Speed Sliders Card
        card_phys = tk.LabelFrame(self, text="  ⚡ KECEPATAN & DURASI RIAK  ", font=("Segoe UI", 8, "bold"), fg=self.c_accent, bg=self.c_card, padx=12, pady=6, highlightthickness=1, highlightbackground=self.c_card_border)
        card_phys.pack(fill="x", padx=16, pady=4)

        # Wave Speed
        f_spd = tk.Frame(card_phys, bg=self.c_card)
        f_spd.pack(fill="x", pady=2)
        self.lbl_spd = tk.Label(f_spd, text=f"Kecepatan Gelombang: {int(self.cfg.get('wave_speed', 0.065) * 1000)} ms / zona", font=("Segoe UI", 8), fg=self.c_text, bg=self.c_card)
        self.lbl_spd.pack(anchor="w")
        self.scale_spd = tk.Scale(f_spd, from_=30, to=150, orient="horizontal", showvalue=False, bg=self.c_card, fg=self.c_accent, highlightthickness=0, troughcolor="#20202e", activebackground=self.c_accent, command=self._on_scale_speed)
        self.scale_spd.set(int(self.cfg.get("wave_speed", 0.065) * 1000))
        self.scale_spd.pack(fill="x")

        # Ripple Duration
        f_dur = tk.Frame(card_phys, bg=self.c_card)
        f_dur.pack(fill="x", pady=2)
        self.lbl_dur = tk.Label(f_dur, text=f"Durasi Pudar Riak: {int(self.cfg.get('ripple_duration', 0.38) * 1000)} ms", font=("Segoe UI", 8), fg=self.c_text, bg=self.c_card)
        self.lbl_dur.pack(anchor="w")
        self.scale_dur = tk.Scale(f_dur, from_=150, to=800, orient="horizontal", showvalue=False, bg=self.c_card, fg=self.c_accent, highlightthickness=0, troughcolor="#20202e", activebackground=self.c_accent, command=self._on_scale_duration)
        self.scale_dur.set(int(self.cfg.get("ripple_duration", 0.38) * 1000))
        self.scale_dur.pack(fill="x")

        # Idle Dimming
        f_dim = tk.Frame(card_phys, bg=self.c_card)
        f_dim.pack(fill="x", pady=2)
        self.lbl_dim = tk.Label(f_dim, text=f"Kecerahan Idle (Redup): {int(self.cfg.get('base_dim_factor', 0.25) * 100)}%", font=("Segoe UI", 8), fg=self.c_text, bg=self.c_card)
        self.lbl_dim.pack(anchor="w")
        self.scale_dim = tk.Scale(f_dim, from_=0, to=100, orient="horizontal", showvalue=False, bg=self.c_card, fg=self.c_accent, highlightthickness=0, troughcolor="#20202e", activebackground=self.c_accent, command=self._on_scale_dim)
        self.scale_dim.set(int(self.cfg.get("base_dim_factor", 0.25) * 100))
        self.scale_dim.pack(fill="x")

        # Hardware Extras (Lightbar)
        f_extras = tk.Frame(card_phys, bg=self.c_card)
        f_extras.pack(fill="x", pady=2)
        self.var_lightbar = tk.BooleanVar(value=self.cfg.get("lightbar_enabled", True))
        chk_bar = tk.Checkbutton(f_extras, text="Aktifkan Pantulan di Lightbar Depan (Bawah Laptop)", variable=self.var_lightbar, font=("Segoe UI", 9), fg=self.c_text, bg=self.c_card, selectcolor="#20202c", activebackground=self.c_card, activeforeground=self.c_text, command=self._on_param_change)
        chk_bar.pack(anchor="w")

        # Footer Notice
        ftr = tk.Label(self, text="⚡ Perubahan langsung tersimpan & diterapkan otomatis ke keyboard fisik dalam 1 detik.", font=("Segoe UI", 8, "italic"), fg=self.c_muted, bg=self.c_bg)
        ftr.pack(side="bottom", pady=6)

    def _rgb_to_hex(self, rgb):
        return f"#{int(max(0, min(255, rgb[0]))):02x}{int(max(0, min(255, rgb[1]))):02x}{int(max(0, min(255, rgb[2]))):02x}"

    def _hex_to_rgb(self, hex_str):
        hex_str = hex_str.lstrip('#')
        return [int(hex_str[i:i+2], 16) for i in (0, 2, 4)]

    def _trigger_ui_ripple(self, zone):
        with self.ripple_lock:
            if len(self.active_ripples) >= 8:
                self.active_ripples.pop(0)

            if self.cfg.get("rainbow_mode", False):
                import colorsys
                self.rainbow_step = (self.rainbow_step + 0.12) % 1.0
                rgb = colorsys.hsv_to_rgb(self.rainbow_step, 1.0, 1.0)
                rip_c = [int(rgb[0] * 255), int(rgb[1] * 255), int(rgb[2] * 255)]
            else:
                rip_c = list(self.cfg.get("custom_ripple_color", [0, 240, 255]))

            self.active_ripples.append({
                'origin': zone,
                't0': time.time(),
                'color': rip_c
            })

    def _start_key_listener(self):
        """Asynchronous key poller for UI Visualizer synchronization."""
        def poller():
            keys_to_poll = list(VK_ZONE_MAP.items())
            last_states = {}
            while self.running_listener:
                for vk, zone in keys_to_poll:
                    state = user32.GetAsyncKeyState(vk)
                    is_down = bool(state & 0x8000)
                    was_down = last_states.get(vk, False)
                    if is_down and not was_down:
                        self._trigger_ui_ripple(zone)
                    last_states[vk] = is_down
                time.sleep(0.015)

        t = threading.Thread(target=poller, daemon=True, name="VisualizerKeyPoller")
        t.start()

    def _on_canvas_click(self, event):
        # Clickable zones on visualizer canvas
        x = event.x
        # 4 zones across canvas width (490 px)
        zone_w = (self.canvas_w - 20) / 4.0
        if 10 <= x <= self.canvas_w - 10:
            zone = int((x - 10) / zone_w)
            zone = max(0, min(3, zone))
            self._trigger_ui_ripple(zone)

    def _animate_visualizer(self):
        """Renders live keyboard deck & lightbar at 35 FPS."""
        now = time.time()
        wave_spd = float(self.cfg.get("wave_speed", 0.065))
        rip_dur = float(self.cfg.get("ripple_duration", 0.38))
        lightbar_on = bool(self.cfg.get("lightbar_enabled", True))
        bar_delay = float(self.cfg.get("lightbar_cascade_delay", 0.035))

        # Base color
        bc = list(self.cfg.get("custom_base_color", [10, 0, 35]))
        dim_factor = float(self.cfg.get("base_dim_factor", 0.25)) if self.cfg.get("dim_base_lighting", True) else 1.0
        base_c = [int(bc[0] * dim_factor), int(bc[1] * dim_factor), int(bc[2] * dim_factor)]

        zone_intensities = [0.0] * 8
        zone_color_sums = [[0.0, 0.0, 0.0] for _ in range(8)]

        with self.ripple_lock:
            surviving = []
            for rip in self.active_ripples:
                origin = rip['origin']
                age = now - rip['t0']
                rip_c = rip.get('color', [0, 240, 255])
                is_alive = False

                for z in range(4):
                    dist = abs(z - origin)
                    arr_time = dist * wave_spd
                    rel_t = age - arr_time
                    if 0.0 <= rel_t <= rip_dur:
                        is_alive = True
                        norm_t = rel_t / rip_dur
                        if norm_t < 0.15:
                            i_val = math.sin((norm_t / 0.15) * (math.pi / 2))
                        else:
                            i_val = math.cos(((norm_t - 0.15) / 0.85) * (math.pi / 2)) ** 2
                        net_i = i_val * (1.0 - dist * 0.08)
                        zone_intensities[z] = min(1.0, zone_intensities[z] + net_i)
                        for ci in range(3):
                            zone_color_sums[z][ci] += rip_c[ci] * net_i

                if lightbar_on:
                    for lz in range(4):
                        dist = abs(lz - origin)
                        arr_time = (dist * wave_spd) + bar_delay
                        rel_t = age - arr_time
                        if 0.0 <= rel_t <= rip_dur:
                            is_alive = True
                            norm_t = rel_t / rip_dur
                            if norm_t < 0.15:
                                i_val = math.sin((norm_t / 0.15) * (math.pi / 2))
                            else:
                                i_val = math.cos(((norm_t - 0.15) / 0.85) * (math.pi / 2)) ** 2
                            net_i = i_val * 0.95 * (1.0 - dist * 0.08)
                            zone_intensities[4 + lz] = min(1.0, zone_intensities[4 + lz] + net_i)
                            for ci in range(3):
                                zone_color_sums[4 + lz][ci] += rip_c[ci] * net_i

                if is_alive or age < (3 * wave_spd + rip_dur + bar_delay):
                    surviving.append(rip)

            self.active_ripples = surviving

        # Compute zone RGBs
        final_colors = []
        for z in range(8):
            factor = zone_intensities[z]
            if factor > 0:
                rc = [min(255, int(zone_color_sums[z][i] / factor)) for i in range(3)]
            else:
                rc = self.cfg.get("custom_ripple_color", [0, 240, 255])
            r = int(base_c[0] + (rc[0] - base_c[0]) * factor)
            g = int(base_c[1] + (rc[1] - base_c[1]) * factor)
            b = int(base_c[2] + (rc[2] - base_c[2]) * factor)
            final_colors.append((r, g, b))

        # Draw on Canvas
        self.canvas.delete("all")

        # 4 Keyboard Zones
        pad = 8
        card_w = (self.canvas_w - 20 - (3 * pad)) / 4.0
        zone_labels = ["WASD / Kiri", "Tengah Kiri", "Tengah Kanan", "Kanan / Num"]

        for i in range(4):
            x1 = 10 + i * (card_w + pad)
            y1 = 8
            x2 = x1 + card_w
            y2 = y1 + 52
            col_hex = self._rgb_to_hex(final_colors[i])
            self.canvas.create_rectangle(x1, y1, x2, y2, fill=col_hex, outline="#333348", width=1)
            self.canvas.create_text(x1 + card_w/2, y1 + 20, text=f"ZONA {i}", font=("Segoe UI", 9, "bold"), fill="#ffffff")
            self.canvas.create_text(x1 + card_w/2, y1 + 36, text=zone_labels[i], font=("Segoe UI", 7), fill="#bbbbcc")

        # Front Wraparound Lightbar
        bar_y1 = 68
        bar_y2 = 82
        bar_pad = 6
        bar_w = (self.canvas_w - 20 - (3 * bar_pad)) / 4.0
        for i in range(4):
            bx1 = 10 + i * (bar_w + bar_pad)
            bx2 = bx1 + bar_w
            b_hex = self._rgb_to_hex(final_colors[4 + i]) if lightbar_on else "#1a1a24"
            self.canvas.create_rectangle(bx1, bar_y1, bx2, bar_y2, fill=b_hex, outline="#2c2c3e", width=1)
        self.canvas.create_text(self.canvas_w/2, bar_y2 + 7, text="▲ BILAH LAMPU DEPAN (FRONT WRAPAROUND LIGHTBAR) ▲", font=("Segoe UI", 6, "bold"), fill=self.c_muted)

        # Loop at ~35 FPS
        self.after(28, self._animate_visualizer)

    def _refresh_status(self):
        active = is_running()
        if active:
            self.lbl_status.config(text="● Status: AKTIF (Terhubung ke Keyboard Fisik)", fg=self.c_green)
            self.btn_toggle.config(text="⏹ MATIKAN", fg=self.c_red)
        else:
            self.lbl_status.config(text="○ Status: NONAKTIF (Lampu Statis)", fg=self.c_muted)
            self.btn_toggle.config(text="▶ NYALAKAN", fg=self.c_green)
        self.after(1500, self._refresh_status)

    def _on_toggle(self):
        subprocess.run([sys.executable, ENGINE_SCRIPT, "--toggle"], capture_output=True)
        self.after(300, self._refresh_status)

    def _on_scale_speed(self, val):
        ms = int(val)
        self.lbl_spd.config(text=f"Kecepatan Gelombang: {ms} ms / zona")
        self.cfg["wave_speed"] = ms / 1000.0
        save_config(self.cfg)

    def _on_scale_duration(self, val):
        ms = int(val)
        self.lbl_dur.config(text=f"Durasi Pudar Riak: {ms} ms")
        self.cfg["ripple_duration"] = ms / 1000.0
        save_config(self.cfg)

    def _on_scale_dim(self, val):
        pct = int(val)
        self.lbl_dim.config(text=f"Kecerahan Idle (Redup): {pct}%")
        self.cfg["base_dim_factor"] = pct / 100.0
        self.cfg["dim_base_lighting"] = (pct < 100)
        save_config(self.cfg)

    def _on_sync_change(self):
        self.cfg["sync_with_ghelper"] = self.var_sync.get()
        save_config(self.cfg)

    def _on_param_change(self):
        self.cfg["rainbow_mode"] = self.var_rainbow.get()
        self.cfg["lightbar_enabled"] = self.var_lightbar.get()
        save_config(self.cfg)

    def _pick_base_color(self):
        curr_hex = self._rgb_to_hex(self.cfg.get("custom_base_color", [10, 0, 35]))
        _, color_hex = colorchooser.askcolor(title="Pilih Warna Dasar (Idle)", initialcolor=curr_hex, parent=self)
        if color_hex:
            rgb = self._hex_to_rgb(color_hex)
            self.cfg["custom_base_color"] = rgb
            self.cfg["sync_with_ghelper"] = False
            self.var_sync.set(False)
            self.btn_c1.config(bg=color_hex)
            save_config(self.cfg)

    def _pick_ripple_color(self):
        curr_hex = self._rgb_to_hex(self.cfg.get("custom_ripple_color", [0, 240, 255]))
        _, color_hex = colorchooser.askcolor(title="Pilih Warna Riak (Ketikan)", initialcolor=curr_hex, parent=self)
        if color_hex:
            rgb = self._hex_to_rgb(color_hex)
            self.cfg["custom_ripple_color"] = rgb
            self.cfg["sync_with_ghelper"] = False
            self.cfg["use_ghelper_color2_as_ripple"] = False
            self.cfg["rainbow_mode"] = False
            self.var_sync.set(False)
            self.var_rainbow.set(False)
            self.btn_c2.config(bg=color_hex)
            save_config(self.cfg)

    def _apply_preset(self, name):
        p = PRESETS.get(name)
        if not p:
            return
        self.cfg.update(p)
        self.cfg["use_ghelper_color2_as_ripple"] = False
        save_config(self.cfg)

        # Update visual controls
        self.var_sync.set(self.cfg.get("sync_with_ghelper", False))
        self.var_rainbow.set(self.cfg.get("rainbow_mode", False))
        self.btn_c1.config(bg=self._rgb_to_hex(self.cfg.get("custom_base_color", [10, 0, 35])))
        self.btn_c2.config(bg=self._rgb_to_hex(self.cfg.get("custom_ripple_color", [0, 240, 255])))
        self.scale_spd.set(int(self.cfg.get("wave_speed", 0.065) * 1000))
        self.scale_dur.set(int(self.cfg.get("ripple_duration", 0.38) * 1000))
        self.scale_dim.set(int(self.cfg.get("base_dim_factor", 0.25) * 100))

        # Fire test preview ripple across zones
        self._trigger_ui_ripple(0)


if __name__ == "__main__":
    app = RippleControlPanel()
    app.mainloop()
