import sys
sys.path.append(r"C:\Users\ROG\AuraZoneRipple\research")
from disas_key_methods import disas

lines = disas(315828, 991)
for l in lines:
    if "0x040005b2" in l or "0x040005b4" in l:
        print(l)
