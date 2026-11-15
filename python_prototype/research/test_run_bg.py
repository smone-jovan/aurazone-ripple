import subprocess
import time
import sys
import os

pythonw = r"C:\Users\ROG\AppData\Local\Programs\Python\Python313\pythonw.exe"
script = r"C:\Users\ROG\AuraZoneRipple\rog_aura_ripple.py"

flags = subprocess.DETACHED_PROCESS | subprocess.CREATE_NEW_PROCESS_GROUP

p = subprocess.Popen([pythonw, script], creationflags=flags, close_fds=True)
print(f"Detached process launched with PID {p.pid}")
time.sleep(1.0)
print("Launcher script exiting now, background process should survive!")
