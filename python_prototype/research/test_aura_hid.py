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

candidate_paths = [
    r"\\?\hid#vid_0b05&pid_19b6&col04#7&2d1c04e0&0&0003#{4d1e55b2-f16f-11cf-88cb-001111000030}",
    r"\\?\hid#vid_0b05&pid_19b6&col05#7&2d1c04e0&0&0004#{4d1e55b2-f16f-11cf-88cb-001111000030}",
]

for p in candidate_paths:
    h = kernel32.CreateFileW(
        p, GENERIC_READ | GENERIC_WRITE,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
        None, OPEN_EXISTING, 0, None
    )
    err = kernel32.GetLastError()
    print(f"Path: {p[:50]}... Handle: {h}, LastError: {err}")
    if h != -1 and h != 0:
        kernel32.CloseHandle(h)
