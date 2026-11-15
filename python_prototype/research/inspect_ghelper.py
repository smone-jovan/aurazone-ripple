import json

with open(r"C:\Users\ROG\AuraZoneRipple\evidence.json", "r", encoding="utf-8") as f:
    data = json.load(f)

rec = [r for r in data["records"] if r.get("operation") == "inspect_managed_members"][0]
norm = rec.get("normalized_result", {})
methods = norm.get("methods", [])

for m in methods:
    decl = m.get("declaring_type", "")
    if decl in ["GHelper.USB.AsusHid", "GHelper.USB.Aura"]:
        print(f"{decl}::{m.get('name')} (token: {m.get('token')})")
