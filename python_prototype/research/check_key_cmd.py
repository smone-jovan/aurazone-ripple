import json

with open(r"C:\Users\ROG\AuraZoneRipple\evidence.json", "r", encoding="utf-8") as f:
    data = json.load(f)

rec = [r for r in data["records"] if r.get("operation") == "inspect_managed_members"][0]
norm = rec.get("normalized_result", {})
methods = norm.get("methods", [])

m1 = [m for m in methods if m.get("token") == "0x06000e84"][0]
m2 = [m for m in methods if m.get("token") == "0x06000e85"][0]

print("RunKeyCommand body:", m1.get("body", {}).get("file_offset"), m1.get("body", {}).get("il_size"))
print("CustomKey body:", m2.get("body", {}).get("file_offset"), m2.get("body", {}).get("il_size"))
