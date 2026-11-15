import json

with open(r"C:\Users\ROG\AuraZoneRipple\evidence.json", "r", encoding="utf-8") as f:
    data = json.load(f)

rec = [r for r in data["records"] if r.get("operation") == "inspect_managed_members"][0]
norm = rec.get("normalized_result", {})
user_strings = norm.get("user_strings")

print("Type of user_strings:", type(user_strings))
if isinstance(user_strings, dict):
    for k in list(user_strings.keys())[:5]:
        print(k, "->", user_strings[k])
elif isinstance(user_strings, list):
    for item in user_strings[:10]:
        print(item)
