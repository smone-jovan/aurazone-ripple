import ctypes
from ctypes import wintypes
import time

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

print("Opened Col05 successfully!")

def send_feature(buf):
    c_buf = (ctypes.c_char * len(buf)).from_buffer_copy(buf)
    res = hid.HidD_SetFeature(h, c_buf, len(buf))
    return bool(res)

# 1. Direct mode handshake: 0x5D, 0xBC, 0x01 padded to 64 bytes
init_pkt = bytearray(64)
init_pkt[0] = 0x5D
init_pkt[1] = 0xBC
init_pkt[2] = 0x01
res_init = send_feature(init_pkt)
print(f"Direct mode init sent: res={res_init}")
time.sleep(0.05)

# 2. Test 4 distinct zone colors:
# Zone 0: Bright Red (255, 0, 0)
# Zone 1: Bright Green (0, 255, 0)
# Zone 2: Bright Blue (0, 0, 255)
# Zone 3: Bright Magenta (255, 0, 255)
# Lightbar zones 4..7: Cyan / Yellow / White / Orange
zone_colors = [
    (255, 0, 0),     # Zone 0
    (0, 255, 0),     # Zone 1
    (0, 0, 255),     # Zone 2
    (255, 0, 255),   # Zone 3
    (0, 255, 255),   # Zone 4
    (255, 255, 0),   # Zone 5
    (255, 255, 255), # Zone 6
    (255, 128, 0),   # Zone 7
]

# G-Helper 4-Zone mapping: [0, 1, 2, 3, 0, 0, 4, 4, 5, 6, 7, 7]
# packet4ZoneFlipped: [0, 1, 2, 3, 0, 0, 4, 4, 5, 6, 7, 7]
# packet4Zone:        [0, 1, 2, 3, 0, 0, 7, 7, 6, 5, 4, 4]
zone_map = [0, 1, 2, 3, 0, 0, 4, 4, 5, 6, 7, 7]

data_pkt = bytearray(64)
data_pkt[0] = 0x5D
data_pkt[1] = 0xBC
data_pkt[2] = 0x00
data_pkt[3] = 0x01
data_pkt[4] = 0x04
data_pkt[5] = 0x00
data_pkt[6] = 0x00
data_pkt[7] = 0x00
data_pkt[8] = 0x00

for i, z in enumerate(zone_map):
    c = zone_colors[z]
    off = 9 + i * 3
    data_pkt[off]     = c[0]
    data_pkt[off + 1] = c[1]
    data_pkt[off + 2] = c[2]

res_data = send_feature(data_pkt)
print(f"4-Zone Direct color packet sent: res={res_data}")

# Keep it for 3 seconds so user can see it!
print("Lighting is set (Red, Green, Blue, Magenta)! Sleeping 3 seconds...")
time.sleep(3.0)

kernel32.CloseHandle(h)
print("Done!")
