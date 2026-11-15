with open(r"C:\Users\ROG\AuraZoneRipple\rog_aura_ripple.py", "r", encoding="utf-8") as f:
    lines = f.readlines()

for i, line in enumerate(lines):
    if "PID_FILE" in line or "pid" in line.lower():
        print(f"{i+1}: {line.strip()}")
