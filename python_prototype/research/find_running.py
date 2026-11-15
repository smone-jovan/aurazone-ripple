with open(r"C:\Users\ROG\AuraZoneRipple\rog_aura_ripple.py", "r", encoding="utf-8") as f:
    lines = f.readlines()

for i, line in enumerate(lines):
    if "running" in line:
        print(f"{i+1}: {line.strip()}")
