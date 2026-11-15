import os
import ctypes
from ctypes import wintypes

kernel32 = ctypes.windll.kernel32

def is_pid_running(pid):
    # PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
    h = kernel32.OpenProcess(0x1000, False, pid)
    if h:
        exit_code = wintypes.DWORD()
        kernel32.GetExitCodeProcess(h, ctypes.byref(exit_code))
        kernel32.CloseHandle(h)
        return exit_code.value == 259
    return False

current_pid = os.getpid()
fake_pid = 999999
print(f"Current PID {current_pid} running: {is_pid_running(current_pid)}")
print(f"Fake PID {fake_pid} running: {is_pid_running(fake_pid)}")
