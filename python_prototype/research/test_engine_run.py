import subprocess
import time
import sys
import os

proc = subprocess.Popen(
    [sys.executable, r"C:\Users\ROG\AuraZoneRipple\rog_aura_ripple.py"],
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True
)

print(f"Spawned engine PID: {proc.pid}")
time.sleep(2.0)

# Check if process is still alive and healthy
ret = proc.poll()
if ret is None:
    print("Process is running smoothly after 2 seconds!")
    # Terminate
    proc.terminate()
    try:
        stdout, stderr = proc.communicate(timeout=2.0)
        print("Stdout:\n", stdout)
        if stderr:
            print("Stderr:\n", stderr)
    except Exception:
        proc.kill()
else:
    stdout, stderr = proc.communicate()
    print(f"Process exited prematurely with code {ret}")
    print("Stdout:\n", stdout)
    print("Stderr:\n", stderr)

# Clean up
os.system(f"python C:\\Users\\ROG\\AuraZoneRipple\\rog_aura_ripple.py --restore")
