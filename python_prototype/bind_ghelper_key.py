"""
================================================================================
 ASUS ROG Strix - G-Helper Key Binding Extension
 Binds the ROG Physical Hotkey (M5 / Armoury Crate Key) to Toggle Aura Ripple.
 Safely manages G-Helper process lifecycle to ensure config persistence.
================================================================================
"""

import os
import sys
import json
import time
import subprocess

GHELPER_CONFIG_PATH = os.path.expandvars(r"%APPDATA%\GHelper\config.json")
TOGGLE_BAT = r"C:\Users\ROG\AuraZoneRipple\Toggle_Aura_Ripple.bat"
GHELPER_TASK_NAME = r"\GHelper_S-1-5-21-3366275841-1223117643-3717383642-1001"
GHELPER_EXE_PATH = r"C:\G-helpe\GHelper\GHelper.exe"


def is_ghelper_running():
    try:
        p = subprocess.run(["tasklist", "/FI", "IMAGENAME eq GHelper.exe"], capture_output=True, text=True)
        return "GHelper.exe" in p.stdout
    except Exception:
        return False


def stop_ghelper():
    print("Menghentikan G-Helper untuk memuat konfigurasi baru...")
    # First try via Scheduled Task (works cleanly even if elevated)
    try:
        subprocess.run(["schtasks", "/end", "/tn", GHELPER_TASK_NAME], capture_output=True, text=True)
        time.sleep(0.5)
    except Exception:
        pass
    
    if is_ghelper_running():
        try:
            subprocess.run(["taskkill", "/F", "/IM", "GHelper.exe"], capture_output=True, text=True)
            time.sleep(0.5)
        except Exception:
            pass


def start_ghelper():
    print("Memulai ulang G-Helper dengan konfigurasi tombol M5 baru...")
    try:
        res = subprocess.run(["schtasks", "/run", "/tn", GHELPER_TASK_NAME], capture_output=True, text=True)
        if "SUCCESS" in res.stdout or "BERHASIL" in res.stdout:
            time.sleep(1.0)
            return True
    except Exception:
        pass

    # Fallback to direct launch
    if os.path.exists(GHELPER_EXE_PATH):
        try:
            subprocess.Popen([GHELPER_EXE_PATH], creationflags=subprocess.CREATE_NEW_PROCESS_GROUP)
            time.sleep(1.0)
            return True
        except Exception as e:
            print(f"[WARN] Gagal memulai langsung: {e}")
    return False


def bind_m5():
    if not os.path.exists(GHELPER_CONFIG_PATH):
        print(f"[ERROR] Config G-Helper tidak ditemukan di: {GHELPER_CONFIG_PATH}")
        return False

    was_running = is_ghelper_running()
    if was_running:
        stop_ghelper()

    try:
        with open(GHELPER_CONFIG_PATH, "r", encoding="utf-8") as f:
            cfg = json.load(f)

        old_val = cfg.get("m5", "")
        cfg["m5"] = TOGGLE_BAT

        with open(GHELPER_CONFIG_PATH, "w", encoding="utf-8") as f:
            json.dump(cfg, f, indent=2)

        print("[OK] Tombol M5 (Tombol ROG Eye) berhasil dihubungkan ke Toggle Aura Ripple!")
        print(f"     Nilai lama: '{old_val}'")
        print(f"     Nilai baru: '{TOGGLE_BAT}'")

        if was_running:
            start_ghelper()
            time.sleep(1.5)
            # Verify retention
            with open(GHELPER_CONFIG_PATH, "r", encoding="utf-8") as f:
                verify_cfg = json.load(f)
            if verify_cfg.get("m5") == TOGGLE_BAT:
                print("\n[VERIFIKASI SUKSES] Konfigurasi tombol M5 aktif di G-Helper!")
                print("-> Sekarang Anda cukup menekan tombol fisik ROG Eye (M5) di keyboard")
                print("   untuk menyalakan atau mematikan efek riak (Aura Ripple) kapan saja.")
                return True
            else:
                print("\n[PERINGATAN] G-Helper menimpa kembali nilai config. Silakan setel manual di menu 'Extra' G-Helper.")
                return False
        return True

    except Exception as e:
        print(f"[ERROR] Gagal memperbarui config G-Helper: {e}")
        return False


if __name__ == "__main__":
    bind_m5()
