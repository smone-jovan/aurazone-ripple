import ctypes
from ctypes import wintypes
import time

user32 = ctypes.windll.user32
user32.GetAsyncKeyState.restype = ctypes.c_short
user32.GetAsyncKeyState.argtypes = [ctypes.c_int]

# Hold key down
user32.keybd_event(0x41, 0, 0, 0)
time.sleep(0.01)
s1 = user32.GetAsyncKeyState(0x41)
user32.keybd_event(0x41, 0, 2, 0) # Up
time.sleep(0.01)
s2 = user32.GetAsyncKeyState(0x41)

print(f"s1 (down): {hex(s1 & 0xFFFF)}, is_down: {bool(s1 & 0x8000)}")
print(f"s2 (up):   {hex(s2 & 0xFFFF)}, is_down: {bool(s2 & 0x8000)}")
