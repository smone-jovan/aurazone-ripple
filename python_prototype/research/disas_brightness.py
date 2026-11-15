import sys
sys.path.append(r"C:\Users\ROG\AuraZoneRipple\research")
from disas_key_methods import disas

print("--- ApplyBrightness ---")
for l in disas(317048, 66, "fat"):
    print(l)

print("--- DirectBrightness ---")
for l in disas(317128, 84, "fat"):
    print(l)
