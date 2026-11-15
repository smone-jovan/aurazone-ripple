import json

with open(r"C:\Users\ROG\AuraZoneRipple\evidence.json", "r", encoding="utf-8") as f:
    d = json.load(f)["records"][0]["normalized_result"]["methods"]

for m in d:
    if m.get("token") in ["0x06000eb0", "0x06000e83"]:
        print(m.get("token"), m.get("declaring_type"), m.get("name"), m.get("signature"))
