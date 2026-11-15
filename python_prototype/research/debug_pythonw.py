import os
import sys
import time

log_file = r"C:\Users\ROG\AuraZoneRipple\debug_pw.log"
with open(log_file, "w", encoding="utf-8") as f:
    f.write(f"Started debug_pw with sys.executable={sys.executable}, pid={os.getpid()}\n")

try:
    with open(log_file, "a", encoding="utf-8") as f:
        f.write("Importing modules...\n")
    import rog_aura_ripple as rar
    with open(log_file, "a", encoding="utf-8") as f:
        f.write("Imported successfully! Testing AuraDirectDevice...\n")
    dev = rar.AuraDirectDevice()
    with open(log_file, "a", encoding="utf-8") as f:
        f.write(f"Aura device opened: {dev.device_path}\n")
    dev.close()
    with open(log_file, "a", encoding="utf-8") as f:
        f.write("Closed device cleanly.\n")
except Exception as e:
    import traceback
    with open(log_file, "a", encoding="utf-8") as f:
        f.write(f"EXCEPTION: {e}\n")
        traceback.print_exc(file=f)
