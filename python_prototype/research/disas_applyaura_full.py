import sys
sys.path.append(r"C:\Users\ROG\AuraZoneRipple\research")
from disas_key_methods import disas

lines = disas(319924, 846)
for l in lines:
    print(l)
