import sys
sys.path.append(r"C:\Users\ROG\AuraZoneRipple\research")
from disas_key_methods import disas

lines = disas(319260, 147)
for l in lines:
    print(l)
