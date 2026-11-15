import sys
sys.path.append(r"C:\Users\ROG\AuraZoneRipple\research")
from disas_key_methods import disas
import json

with open(r"C:\Users\ROG\AuraZoneRipple\research\evidence.json", "r", encoding="utf-8") as f:
    records = json.load(f)["records"]
rec = [r for r in records if r.get("operation") == "inspect_managed_members"][0]
methods = rec["normalized_result"]["methods"]
m = [m for m in methods if m.get("token") == "0x06000462"][0]
b = m.get("body", {})
print("WriteInput:", b)
for l in disas(b.get("file_offset"), b.get("il_size"), b.get("header_format", "fat")):
    print(l)
