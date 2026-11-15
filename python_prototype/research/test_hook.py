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

hook_id = None
key_events = []

def low_level_keyboard_proc(nCode, wParam, lParam):
    if nCode >= 0:
        if wParam in (WM_KEYDOWN, WM_SYSKEYDOWN):
            kb = KBDLLHOOKSTRUCT.from_address(lParam)
            key_events.append(kb.vkCode)
    return user32.CallNextHookEx(hook_id, nCode, wParam, lParam)

callback = HOOKPROC(low_level_keyboard_proc)

def run_test():
    global hook_id
    hook_id = user32.SetWindowsHookExW(
        WH_KEYBOARD_LL,
        callback,
        kernel32.GetModuleHandleW(None),
        0
    )
    print(f"Hook installed: {hook_id != 0} (id={hook_id})")
    
    # Send a synthetic keypress to test hook
    user32.keybd_event(0x41, 0, 0, 0) # 'A' down
    user32.keybd_event(0x41, 0, 2, 0) # 'A' up
    
    msg = wintypes.MSG()
    # Pump messages for 0.1s
    t0 = time.time()
    while time.time() - t0 < 0.2:
        if user32.PeekMessageW(ctypes.byref(msg), None, 0, 0, 1): # PM_REMOVE = 1
            user32.TranslateMessage(ctypes.byref(msg))
            user32.DispatchMessageW(ctypes.byref(msg))
        time.sleep(0.01)
        
    user32.UnhookWindowsHookEx(hook_id)
    print(f"Captured key events: {[hex(k) for k in key_events]}")

if __name__ == "__main__":
    run_test()
