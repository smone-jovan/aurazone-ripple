import json

with open(r"C:\Users\ROG\AuraZoneRipple\research\evidence.json", "r", encoding="utf-8") as f:
    data = json.load(f)

for r in data.get("records", []):
    if r.get("operation") == "inspect_managed_members":
        methods = r.get("normalized_result", {}).get("methods", [])
        for m in methods:
            name = m.get("name", "")
            decl = m.get("declaring_type", "")
            if any(k in name.lower() or k in decl.lower() for k in ["direct", "aura", "lightbar"]):
                body = m.get("body", {})
                print(f"{decl}::{name} -> token {m.get('token')} offset {body.get('file_offset')} size {body.get('il_size')}")
