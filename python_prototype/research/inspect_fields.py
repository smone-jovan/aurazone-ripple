import json

with open(r"C:\Users\ROG\AuraZoneRipple\evidence.json", "r", encoding="utf-8") as f:
    data = json.load(f)

rec = [r for r in data["records"] if r.get("operation") == "inspect_managed_members"][0]
norm = rec.get("normalized_result", {})
fields = norm.get("fields", [])

for f in fields:
    decl = f.get("declaring_type", "")
    if decl in ["GHelper.USB.AsusHid", "GHelper.USB.Aura"]:
        print(f"{decl}::{f.get('name')} : {f.get('signature', {}).get('field_type')} [token: {f.get('token')}]")
