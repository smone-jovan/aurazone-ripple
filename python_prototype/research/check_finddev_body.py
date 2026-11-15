import json

with open(r"C:\Users\ROG\AuraZoneRipple\evidence.json", "r", encoding="utf-8") as f:
    data = json.load(f)

rec = [r for r in data["records"] if r.get("operation") == "inspect_managed_members"][0]
norm = rec.get("normalized_result", {})
methods = norm.get("methods", [])

m = [m for m in methods if m.get("token") == "0x06000460"][0]
body = m.get("body", {})
print("FindDevices body:")
print("file_offset:", body.get("file_offset"), "il_size:", body.get("il_size"), "header:", body.get("header_format"))
