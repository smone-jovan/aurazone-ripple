import sys
sys.path.append(r"C:\Users\ROG\AuraZoneRipple\research")
from disas_key_methods import disas

lines = disas(1796, 105)
for l in lines:
    print(l)
