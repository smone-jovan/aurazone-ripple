import winreg
import ctypes
from ctypes import wintypes
import time
import json
import os

kernel32 = ctypes.windll.kernel32
hid = ctypes.windll.hid

GENERIC_READ = 0x80000000
GENERIC_WRITE = 0x40000000
FILE_SHARE_READ = 1
FILE_SHARE_WRITE = 2
OPEN_EXISTING = 3

class HIDP_CAPS(ctypes.Structure):
    _fields_ = [
        ("Usage", wintypes.USHORT),
        ("UsagePage", wintypes.USHORT),
        ("InputReportByteLength", wintypes.USHORT),
        ("OutputReportByteLength", wintypes.USHORT),
        ("FeatureReportByteLength", wintypes.USHORT),
        ("Reserved", wintypes.USHORT * 17),
        ("NumberLinkCollectionNodes", wintypes.USHORT),
        ("NumberInputButtonCaps", wintypes.USHORT),
        ("NumberInputValueCaps", wintypes.USHORT),
        ("NumberInputDataIndices", wintypes.USHORT),
        ("NumberOutputButtonCaps", wintypes.USHORT),
        ("NumberOutputValueCaps", wintypes.USHORT),
        ("NumberOutputDataIndices", wintypes.USHORT),
        ("NumberFeatureButtonCaps", wintypes.USHORT),
        ("NumberFeatureValueCaps", wintypes.USHORT),
        ("NumberFeatureDataIndices", wintypes.USHORT),
    ]

def find_col05_path():
    base_key = r"SYSTEM\CurrentControlSet\Control\DeviceClasses\{4d1e55b2-f16f-11cf-88cb-001111000030}"
    try:
        with winreg.OpenKey(winreg.HKEY_LOCAL_MACHINE, base_key) as k:
            i = 0
            while True:
                try:
                    sub = winreg.EnumKey(k, i)
                    i += 1
                    if "0b05" in sub.lower() and "col05" in sub.lower():
                        sym_path = "\\\\?\\" + sub[4:]
                        return sym_path
                except OSError:
                    break
    except Exception as e:
        print(f"Registry enum failed: {e}")
    # Fallback to known path
    return r"\\?\HID#VID_0B05&PID_19B6&Col05#7&2d1c04e0&0&0004#{4d1e55b2-f16f-11cf-88cb-001111000030}"

def get_ghelper_color():
    cfg_path = os.path.expandvars(r"%APPDATA%\GHelper\config.json")
    if os.path.exists(cfg_path):
        try:
            with open(cfg_path, "r", encoding="utf-8") as f:
                d = json.load(f)
            c1 = d.get("aura_color", -16776961) & 0xFFFFFFFF
            r = (c1 >> 16) & 0xFF
            g = (c1 >> 8) & 0xFF
            b = c1 & 0xFF
            bright = d.get("keyboard_brightness", 3)
            scale = [0.0, 0.33, 0.66, 1.0][min(3, max(0, bright))]
            return (int(r * scale), int(g * scale), int(b * scale))
        except Exception:
            pass
    return (0, 0, 255)

def restore_static_color():
    path = find_col05_path()
    h = kernel32.CreateFileW(
        path, GENERIC_READ | GENERIC_WRITE,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
        None, OPEN_EXISTING, 0, None
    )
    if h == -1 or h == 0:
        print(f"Failed to open device: {kernel32.GetLastError()}")
        return False

    def send_feature(buf):
        c_buf = (ctypes.c_char * len(buf)).from_buffer_copy(buf)
        return bool(hid.HidD_SetFeature(h, c_buf, len(buf)))

    # Init
    init_pkt = bytearray(64)
    init_pkt[0] = 0x5D
    init_pkt[1] = 0xBC
    init_pkt[2] = 0x01
    send_feature(init_pkt)
    time.sleep(0.05)

    base = get_ghelper_color()
    print(f"Restoring static G-Helper color: RGB{base}")

    pkt = bytearray(64)
    pkt[0] = 0x5D
    pkt[1] = 0xBC
    pkt[2] = 0x00
    pkt[3] = 0x01
    pkt[4] = 0x04
    # Fill 12 zone slots with base color
    for i in range(12):
        off = 9 + i * 3
        pkt[off]     = base[0]
        pkt[off + 1] = base[1]
        pkt[off + 2] = base[2]
    
    res = send_feature(pkt)
    kernel32.CloseHandle(h)
    print(f"Static color restored: {res}")
    return res

if __name__ == "__main__":
    restore_static_color()
