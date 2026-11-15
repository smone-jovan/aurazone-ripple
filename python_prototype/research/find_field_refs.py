import json

with open(r"C:\Users\ROG\AuraZoneRipple\research\evidence.json", "r", encoding="utf-8") as f:
    records = json.load(f)["records"]

rec = [r for r in records if r.get("operation") == "inspect_managed_members"][0]
fields = rec["normalized_result"]["fields"]
fld_map = {f["token"]: f["name"] for f in fields}

methods = rec["normalized_result"]["methods"]
for m in methods:
    for c in m.get("field_accesses", []):
        if c in ["0x040005b2", "0x040005b4", "0x040005c2", "0x040005c3"]:
            print(f"{m.get('declaring_type')}::{m.get('name')} accesses {fld_map.get(c, c)}")
