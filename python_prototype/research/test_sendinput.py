import ctypes
from ctypes import wintypes
import time

user32 = ctypes.windll.user32
kernel32 = ctypes.windll.kernel32

INPUT_KEYBOARD = 1
KEYEVENTF_KEYUP = 2

class KEYBDINPUT(ctypes.Structure):
    _fields_ = [
        ("wVk", wintypes.WORD),
        ("wScan", wintypes.WORD),
        ("dwFlags", wintypes.DWORD),
        ("time", wintypes.DWORD),
        ("dwExtraInfo", ctypes.c_size_t)
    ]

class INPUT(ctypes.Structure):
    class _INPUT_UNION(ctypes.Union):
        _fields_ = [("ki", KEYBDINPUT)]
    _anonymous_ = ("_u",)
    _fields_ = [
        ("type", wintypes.DWORD),
        ("_u", _INPUT_UNION)
    ]

user32.SendInput.argtypes = [wintypes.UINT, ctypes.POINTER(INPUT), ctypes.c_int]
user32.SendInput.restype = wintypes.UINT
user32.GetAsyncKeyState.restype = ctypes.c_short
user32.GetAsyncKeyState.argtypes = [ctypes.c_int]

inp = INPUT()
inp.type = INPUT_KEYBOARD
inp.ki.wVk = 0x41

res1 = user32.SendInput(1, ctypes.byref(inp), ctypes.sizeof(INPUT))
time.sleep(0.01)
s1 = user32.GetAsyncKeyState(0x41)

inp.ki.dwFlags = KEYEVENTF_KEYUP
res2 = user32.SendInput(1, ctypes.byref(inp), ctypes.sizeof(INPUT))
time.sleep(0.01)
s2 = user32.GetAsyncKeyState(0x41)

print(f"SendInput res1: {res1}, res2: {res2}")
print(f"s1: {hex(s1 & 0xFFFF)}, s2: {hex(s2 & 0xFFFF)}")
