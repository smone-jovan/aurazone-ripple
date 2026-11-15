import json

with open(r"C:\Users\ROG\AuraZoneRipple\evidence.json", "r", encoding="utf-8") as f:
    methods = json.load(f)["records"][0]["normalized_result"]["methods"]

for m in methods:
    if "aura" in m.get("declaring_type", "").lower() or "aura" in m.get("name", "").lower():
        print(f"{m.get('declaring_type')}::{m.get('name')} ({m.get('token')})")
