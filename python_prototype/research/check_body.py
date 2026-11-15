import json

with open(r"C:\Users\ROG\AuraZoneRipple\evidence.json", "r", encoding="utf-8") as f:
    data = json.load(f)

rec = [r for r in data["records"] if r.get("operation") == "inspect_managed_members"][0]
norm = rec.get("normalized_result", {})
methods = norm.get("methods", [])

m_direct = [m for m in methods if m.get("token") == "0x060004a7"][0]
print("Body keys:", m_direct.get("body", {}).keys())
for k, v in m_direct.get("body", {}).items():
    if k != "anchors":
        print(f"{k}: {v}")
