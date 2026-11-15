import ctypes
from ctypes import wintypes
import threading
import time

user32 = ctypes.windll.user32
kernel32 = ctypes.windll.kernel32

WH_KEYBOARD_LL = 13
WM_KEYDOWN = 0x0100
WM_SYSKEYDOWN = 0x0104

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
        vk = ctypes.c_ulong.from_address(lParam).value
        caught.append(vk)
    return user32.CallNextHookEx(hook_h, code, wParam, lParam)

cb = HOOKPROC(proc)

thread_id = None
def hook_thread_func():
    global hook_h, thread_id
    thread_id = kernel32.GetCurrentThreadId()
    # Install hook with hMod=None
    hook_h = user32.SetWindowsHookExW(WH_KEYBOARD_LL, cb, None, 0)
    print(f"Hook thread started, id={thread_id}, hook_h={hook_h}, lastErr={kernel32.GetLastError()}")
    
    msg = wintypes.MSG()
    while user32.GetMessageW(ctypes.byref(msg), 0, 0, 0) > 0:
        user32.TranslateMessage(ctypes.byref(msg))
        user32.DispatchMessageW(ctypes.byref(msg))
        
    user32.UnhookWindowsHookEx(hook_h)
    print("Hook thread exited")

t = threading.Thread(target=hook_thread_func, daemon=True)
t.start()
time.sleep(0.1)

# Synthesize key
user32.keybd_event(0x41, 0, 0, 0)
time.sleep(0.01)
user32.keybd_event(0x41, 0, 2, 0)
time.sleep(0.1)

print("Caught:", [hex(x) for x in caught])
# Stop thread
user32.PostThreadMessageW(thread_id, 0x0012, 0, 0)
time.sleep(0.1)
