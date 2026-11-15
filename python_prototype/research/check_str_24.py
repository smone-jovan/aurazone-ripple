import json

with open(r"C:\Users\ROG\AuraZoneRipple\evidence.json", "r", encoding="utf-8") as f:
    data = json.load(f)

rec = [r for r in data["records"] if r.get("operation") == "inspect_managed_members"][0]
norm = rec.get("normalized_result", {})
user_strings = norm.get("user_strings", {})

for item in user_strings.get("issues", []):
    pass

# search in blob or user strings
for k in ["0x7000d248", "0x7000cebc", "0x7000cee8"]:
    print(k, ":", user_strings.get(k))
