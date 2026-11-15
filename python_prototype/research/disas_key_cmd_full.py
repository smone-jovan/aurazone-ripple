import sys
sys.path.append(r"C:\Users\ROG\AuraZoneRipple\research")
from disas_key_methods import disas

print("--- RunKeyCommand ---")
for l in disas(474124, 124, "fat"):
    print(l)

print("--- CustomKey ---")
for l in disas(474276, 24, "fat"):
    print(l)
