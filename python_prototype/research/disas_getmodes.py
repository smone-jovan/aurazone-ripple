import json
from disas_il import disas

with open(r"C:\Users\ROG\AuraZoneRipple\evidence.json", "r", encoding="utf-8") as f:
    data = json.load(f)

rec = [r for r in data["records"] if r.get("operation") == "inspect_managed_members"][0]
norm = rec.get("normalized_result", {})
methods = norm.get("methods", [])

m = [m for m in methods if m.get("token") == "0x06000491"][0]
b = m.get("body", {})
print("GetModes body:", b.get("file_offset"), b.get("il_size"))
lines = disas(b.get("file_offset"), b.get("il_size"), b.get("header_format", "fat"))
for l in lines:
    print(l)
