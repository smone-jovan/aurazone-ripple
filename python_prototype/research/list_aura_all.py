import json

with open(r"C:\Users\ROG\AuraZoneRipple\research\evidence.json", "r", encoding="utf-8") as f:
    records = json.load(f)["records"]

rec = [r for r in records if r.get("operation") == "inspect_managed_members"][0]
methods = rec["normalized_result"]["methods"]
for m in methods:
    if m.get("declaring_type") == "GHelper.USB.Aura":
        b = m.get("body", {})
        print(f"{m.get('token')} {m.get('name')} offset: {b.get('file_offset')} size: {b.get('il_size')}")
