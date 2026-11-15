import json

with open(r"C:\Users\ROG\AuraZoneRipple\evidence.json", "r", encoding="utf-8") as f:
    methods = json.load(f)["records"][3]["normalized_result"]["methods"]

kb_methods = [m for m in methods if "keyboardhook" in m.get("declaring_type", "").lower() or "keyboard" in m.get("name", "").lower()]

for m in kb_methods:
    print(m.get("declaring_type"), m.get("name"), m.get("token"), m.get("body", {}).get("file_offset"), m.get("body", {}).get("il_size"))
