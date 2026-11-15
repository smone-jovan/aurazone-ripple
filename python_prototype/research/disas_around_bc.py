import sys
sys.path.append(r"C:\Users\ROG\AuraZoneRipple\research")
from disas_key_methods import disas

lines = disas(318404, 842)
for i, l in enumerate(lines):
    if "0x000000bc" in l or "188" in l:
        for j in range(max(0, i-15), min(len(lines), i+30)):
            print(lines[j])
        print("-----------------------------------------")
