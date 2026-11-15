import ctypes
from ctypes import wintypes
import os

kernel32 = ctypes.windll.kernel32

kernel32.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
kernel32.OpenProcess.restype = wintypes.HANDLE

kernel32.GetExitCodeProcess.argtypes = [wintypes.HANDLE, ctypes.POINTER(wintypes.DWORD)]
kernel32.GetExitCodeProcess.restype = wintypes.BOOL

kernel32.CloseHandle.argtypes = [wintypes.HANDLE]
kernel32.CloseHandle.restype = wintypes.BOOL

def check(pid):
    h = kernel32.OpenProcess(0x1000, False, pid)
    err = kernel32.GetLastError()
    print(f"OpenProcess: handle={h}, err={err}")
    if h:
        code = wintypes.DWORD()
        res = kernel32.GetExitCodeProcess(h, ctypes.byref(code))
        kernel32.CloseHandle(h)
        print(f"GetExitCodeProcess res={res}, code={code.value}")
        return code.value == 259
    return False

pid = os.getpid()
print(f"PID {pid} running: {check(pid)}")
