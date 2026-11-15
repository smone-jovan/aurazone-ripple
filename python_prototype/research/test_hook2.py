import ctypes
from ctypes import wintypes

user32 = ctypes.windll.user32
kernel32 = ctypes.windll.kernel32

WH_KEYBOARD_LL = 13
HOOKPROC = ctypes.WINFUNCTYPE(ctypes.c_ssize_t, ctypes.c_int, wintypes.WPARAM, wintypes.LPARAM)

user32.SetWindowsHookExW.argtypes = [ctypes.c_int, HOOKPROC, wintypes.HINSTANCE, wintypes.DWORD]
user32.SetWindowsHookExW.restype = wintypes.HHOOK

def proc(code, w, l):
    return user32.CallNextHookEx(None, code, w, l)

cb = HOOKPROC(proc)
h = user32.SetWindowsHookExW(WH_KEYBOARD_LL, cb, None, 0)
err = kernel32.GetLastError()
print(f"Hook handle: {h}, error: {err}")
if h:
    user32.UnhookWindowsHookEx(h)
