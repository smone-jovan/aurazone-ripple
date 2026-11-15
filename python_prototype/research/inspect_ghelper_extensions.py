import json

with open(r"C:\Users\ROG\AuraZoneRipple\evidence.json", "r", encoding="utf-8") as f:
    data = json.load(f)

rec = [r for r in data["records"] if r.get("operation") == "inspect_managed_members"][0]
norm = rec.get("normalized_result", {})
methods = norm.get("methods", [])
user_strings = norm.get("user_strings", {})

for m in methods:
    name = m.get("name", "")
    decl = m.get("declaring_type", "")
    if any(k in name.lower() for k in ["custom", "script", "command", "action", "hook", "pipe", "plugin", "run"]):
        print(f"{decl}::{name} -> token: {m.get('token')}")
