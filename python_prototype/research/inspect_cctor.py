import json

with open(r"C:\Users\ROG\AuraZoneRipple\evidence.json", "r", encoding="utf-8") as f:
    data = json.load(f)

rec = [r for r in data["records"] if r.get("operation") == "inspect_managed_members"][0]
norm = rec.get("normalized_result", {})
methods = norm.get("methods", [])
fields = {f.get("token"): f.get("name") for f in norm.get("fields", [])}

for m in methods:
    if m.get("name") == ".cctor" and m.get("declaring_type") in ["GHelper.USB.AsusHid", "GHelper.USB.Aura"]:
        print(f"\n==================== {m.get('declaring_type')}::.cctor ====================")
        anchors = m.get("body", {}).get("anchors", [])
        for a in anchors:
            op = a.get("opcode")
            val = a.get("operand", "")
            if val in fields:
                val = f"{val} ({fields[val]})"
            print(f"  {a.get('il_offset'):04x}: {op} {val}")
