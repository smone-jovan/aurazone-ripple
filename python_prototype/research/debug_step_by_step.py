import sys
import os
import time
import traceback
sys.path.append(r"C:\Users\ROG\AuraZoneRipple")

import rog_aura_ripple as rar

print("Step 1: Open Aura")
try:
    aura = rar.AuraDirectDevice()
    print("Aura opened:", aura.handle)
except Exception as e:
    print("Aura failed:", e)
    traceback.print_exc()
    sys.exit(1)

settings = rar.load_settings()
engine = rar.ReactiveRippleEngine(aura, settings)
print("Engine created, running=", engine.running)

# Now test engine.run in a thread with error catching
def safe_run():
    try:
        engine.run()
    except Exception as e:
        print("[CRASH] RenderEngine exception:", e)
        traceback.print_exc()

import threading
t_render = threading.Thread(target=safe_run, daemon=True, name="Render")
t_render.start()
print("Render thread started")
time.sleep(1.0)
print("Render thread alive after 1s:", t_render.is_alive(), "engine.running:", engine.running)

# Now test hook
def safe_hook():
    try:
        rar.start_keystroke_hook(engine)
    except Exception as e:
        print("[CRASH] Hook exception:", e)
        traceback.print_exc()

safe_hook()
print("Hook thread started")
time.sleep(1.0)
print("Engine.running after hook:", engine.running)

# Now test poller
def safe_poller():
    try:
        rar.start_asynckeystate_poller(engine)
    except Exception as e:
        print("[CRASH] Poller exception:", e)
        traceback.print_exc()

safe_poller()
print("Poller thread started")
time.sleep(1.0)
print("Engine.running after poller:", engine.running)

# Now wait 3 seconds and check
for i in range(3):
    time.sleep(1)
    print(f"Second {i+1}: engine.running={engine.running}, t_render.is_alive={t_render.is_alive()}")

print("Cleaning up...")
engine.running = False
time.sleep(0.1)
aura.close()
print("Done!")
