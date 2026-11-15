import json

with open(r"C:\Users\ROG\AuraZoneRipple\evidence.json", "r", encoding="utf-8") as f:
    data = json.load(f)

for i, r in enumerate(data["records"]):
    print(i, r.get("operation"), list(r.get("normalized_result", {}).keys()))
