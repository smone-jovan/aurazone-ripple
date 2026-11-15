import ctypes
from ctypes import wintypes
import time
import math

kernel32 = ctypes.windll.kernel32
hid = ctypes.windll.hid

GENERIC_READ = 0x80000000
GENERIC_WRITE = 0x40000000
FILE_SHARE_READ = 1
FILE_SHARE_WRITE = 2
OPEN_EXISTING = 3

col05_path = r"\\?\HID#VID_0B05&PID_19B6&Col05#7&2d1c04e0&0&0004#{4d1e55b2-f16f-11cf-88cb-001111000030}"

h = kernel32.CreateFileW(
    col05_path, GENERIC_READ | GENERIC_WRITE,
    FILE_SHARE_READ | FILE_SHARE_WRITE,
    None, OPEN_EXISTING, 0, None
)

if h == -1 or h == 0:
    print(f"Failed to open Col05, error: {kernel32.GetLastError()}")
    exit(1)

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

# G-Helper 4-Zone map for Strix:
# 12 entries: zones 0, 1, 2, 3 (keyboard left to right) and 4, 5, 6, 7 (lightbar)
zone_map = [0, 1, 2, 3, 0, 0, 4, 4, 5, 6, 7, 7]

def set_zones(colors):
    pkt = bytearray(64)
    pkt[0] = 0x5D
    pkt[1] = 0xBC
    pkt[2] = 0x00
    pkt[3] = 0x01
    pkt[4] = 0x04
    pkt[5] = 0x00
    pkt[6] = 0x00
    pkt[7] = 0x00
    pkt[8] = 0x00
    for i, z in enumerate(zone_map):
        c = colors[z]
        off = 9 + i * 3
        pkt[off]     = int(c[0])
        pkt[off + 1] = int(c[1])
        pkt[off + 2] = int(c[2])
    return send_feature(pkt)

print("Starting smooth 4-zone wave test for 3 seconds...")
t0 = time.time()
while time.time() - t0 < 3.0:
    elapsed = time.time() - t0
    colors = []
    # 4 keyboard zones + 4 lightbar zones
    for z in range(8):
        # Calculate rainbow wave phase
        phase = elapsed * 4.0 - z * 0.8
        r = int((math.sin(phase) + 1.0) * 127.5)
        g = int((math.sin(phase + 2.094) + 1.0) * 127.5)
        b = int((math.sin(phase + 4.188) + 1.0) * 127.5)
        colors.append((r, g, b))
    set_zones(colors)
    time.sleep(0.02) # ~50 FPS

print("Wave test finished successfully!")
kernel32.CloseHandle(h)
