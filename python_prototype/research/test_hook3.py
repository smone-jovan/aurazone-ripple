import ctypes
from ctypes import wintypes
import time

user32 = ctypes.windll.user32
kernel32 = ctypes.windll.kernel32

WH_KEYBOARD_LL = 13
WM_KEYDOWN = 0x0100
WM_SYSKEYDOWN = 0x0104

class KBDLLHOOKSTRUCT(ctypes.Structure):
    _fields_ = [
        ("vkCode", wintypes.DWORD),
        ("scanCode", wintypes.DWORD),
        ("flags", wintypes.DWORD),
        ("time", wintypes.DWORD),
        ("dwExtraInfo", ctypes.c_size_t)
    ]

HOOKPROC = ctypes.WINFUNCTYPE(ctypes.c_ssize_t, ctypes.c_int, wintypes.WPARAM, wintypes.LPARAM)

user32.SetWindowsHookExW.argtypes = [ctypes.c_int, HOOKPROC, wintypes.HINSTANCE, wintypes.DWORD]
user32.SetWindowsHookExW.restype = wintypes.HHOOK
user32.UnhookWindowsHookEx.argtypes = [wintypes.HHOOK]
user32.UnhookWindowsHookEx.restype = wintypes.BOOL
user32.CallNextHookEx.argtypes = [wintypes.HHOOK, ctypes.c_int, wintypes.WPARAM, wintypes.LPARAM]
user32.CallNextHookEx.restype = ctypes.c_ssize_t

hook_h = None
caught = []

def proc(code, wParam, lParam):
    if code >= 0 and wParam in (WM_KEYDOWN, WM_SYSKEYDOWN):
        kb = KBDLLHOOKSTRUCT.from_address(lParam)
        caught.append(kb.vkCode)
    return user32.CallNextHookEx(hook_h, code, wParam, lParam)

cb = HOOKPROC(proc)
hook_h = user32.SetWindowsHookExW(WH_KEYBOARD_LL, cb, None, 0)
print("Hooked:", hook_h)

# Synthesize keypress
user32.keybd_event(0x41, 0, 0, 0) # A down
user32.keybd_event(0x41, 0, 2, 0) # A up

msg = wintypes.MSG()
t0 = time.time()
while time.time() - t0 < 0.2:
    if user32.PeekMessageW(ctypes.byref(msg), None, 0, 0, 1):
        user32.TranslateMessage(ctypes.byref(msg))
        user32.DispatchMessageW(ctypes.byref(msg))
    time.sleep(0.01)

user32.UnhookWindowsHookEx(hook_h)
print("Caught:", [hex(x) for x in caught])
