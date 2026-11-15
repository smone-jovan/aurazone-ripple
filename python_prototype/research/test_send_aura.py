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

paths = [
    r"\\?\HID#VID_0B05&PID_19B6&Col04#7&2d1c04e0&0&0003#{4d1e55b2-f16f-11cf-88cb-001111000030}",
    r"\\?\HID#VID_0B05&PID_19B6&Col05#7&2d1c04e0&0&0004#{4d1e55b2-f16f-11cf-88cb-001111000030}",
]

for p in paths:
    print(f"\nTesting path: {p.split('#')[1]}")
    h = kernel32.CreateFileW(
        p, GENERIC_READ | GENERIC_WRITE,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
        None, OPEN_EXISTING, 0, None
    )
    if h == -1 or h == 0:
        print(f"  Failed to open: error {kernel32.GetLastError()}")
        continue
    print(f"  Handle: {h}")

    # Test 1: Send 3-byte init pkt
    init_3 = bytearray([0x5D, 0xBC, 0x01])
    c_buf3 = (ctypes.c_char * len(init_3)).from_buffer_copy(init_3)
    res3 = hid.HidD_SetFeature(h, c_buf3, len(init_3))
    err3 = kernel32.GetLastError()
    print(f"  Test 1 (3-byte init): res={bool(res3)}, err={err3}")

    # Test 2: Send 64-byte init pkt
    init_64 = bytearray(64)
    init_64[0] = 0x5D
    init_64[1] = 0xBC
    init_64[2] = 0x01
    c_buf64 = (ctypes.c_char * len(init_64)).from_buffer_copy(init_64)
    res64 = hid.HidD_SetFeature(h, c_buf64, len(init_64))
    err64 = kernel32.GetLastError()
    print(f"  Test 2 (64-byte init): res={bool(res64)}, err={err64}")

    # Test 3: Send 4-zone Direct color packet (Red, Green, Blue, Yellow)
    # GHelper packet format for 4-zone:
    # loc.0[0] = 0x5D
    # loc.0[1] = 0xBC
    # loc.0[2] = 0x00
    # loc.0[3] = 0x01
    # loc.0[4] = 0x04
    # loc.0[5] = 0x00
    # loc.0[6] = 0x00
    # loc.0[7] = 0x00
    # loc.0[8] = 0x00
    # Colors at offset 9: 12 zones * 3 = 36 bytes
    # packet4Zone = [0, 1, 2, 3, 0, 0, 7, 7, 6, 5, 4, 4]
    # packet4ZoneFlipped = [0, 1, 2, 3, 0, 0, 4, 4, 5, 6, 7, 7]
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

    zone_map = [0, 1, 2, 3, 0, 0, 4, 4, 5, 6, 7, 7]
    test_colors = [
        (255, 0, 0),    # Zone 0: Red
        (0, 255, 0),    # Zone 1: Green
        (0, 0, 255),    # Zone 2: Blue
        (255, 255, 0),  # Zone 3: Yellow
        (255, 0, 255),  # Zone 4
        (0, 255, 255),  # Zone 5
        (255, 128, 0),  # Zone 6
        (128, 0, 255),  # Zone 7
    ]
    for i, z in enumerate(zone_map):
        c = test_colors[z]
        off = 9 + i * 3
        pkt[off] = c[0]
        pkt[off+1] = c[1]
        pkt[off+2] = c[2]

    c_pkt = (ctypes.c_char * len(pkt)).from_buffer_copy(pkt)
    res_dir = hid.HidD_SetFeature(h, c_pkt, len(pkt))
    err_dir = kernel32.GetLastError()
    print(f"  Test 3 (64-byte 4-zone direct): res={bool(res_dir)}, err={err_dir}")

    kernel32.CloseHandle(h)
