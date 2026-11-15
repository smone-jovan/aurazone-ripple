import subprocess
import time
import sys
import os

pythonw = r"C:\Users\ROG\AppData\Local\Programs\Python\Python313\pythonw.exe"
script = r"C:\Users\ROG\AuraZoneRipple\rog_aura_ripple.py"
# CREATE_BREAKAWAY_FROM_JOB = 0x01000000
flags = 0x01000000 | 0x08000000 | 0x00000200

try:
    p = subprocess.Popen([pythonw, script], creationflags=flags, close_fds=True)
    print(f"Spawned with breakaway PID: {p.pid}")
except Exception as e:
    print(f"Breakaway failed (job doesn't allow): {e}")
