import os
import sys
import time
import subprocess
import rog_aura_ripple as rar

print("Initial get_running_pid():", rar.get_running_pid())

pythonw = r"C:\Users\ROG\AppData\Local\Programs\Python\Python313\pythonw.exe"
script = r"C:\Users\ROG\AuraZoneRipple\rog_aura_ripple.py"
flags = 0x08000000 | 0x00000008 | 0x00000200

p = subprocess.Popen([pythonw, script], creationflags=flags, close_fds=True)
print(f"Spawned PID: {p.pid}")

for i in range(5):
    time.sleep(0.5)
    exists = os.path.exists(rar.PID_FILE)
    content = open(rar.PID_FILE).read().strip() if exists else "MISSING"
    r_pid = rar.get_running_pid()
    alive = rar.is_pid_running(p.pid)
    print(f"[{i*0.5}s] file_exists={exists}, file_content={content}, get_running_pid={r_pid}, is_pid_running({p.pid})={alive}")
