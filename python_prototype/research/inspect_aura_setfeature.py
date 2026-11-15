import json

with open(r"C:\Users\ROG\AuraZoneRipple\evidence.json", "r", encoding="utf-8") as f:
    data = json.load(f)

rec = [r for r in data["records"] if r.get("operation") == "inspect_managed_members"][0]
norm = rec.get("normalized_result", {})
methods = norm.get("methods", [])
fields = {f.get("token"): f.get("name") for f in norm.get("fields", [])}
user_strings = norm.get("user_strings", {})

tokens_to_inspect = {
    "0x06000466": "SetFeatureAura",
    "0x060004a7": "ApplyDirect_1",
    "0x060004a9": "ApplyDirect_2",
    "0x0600046f": "IsDirectReady/Supported?",
}

for m in methods:
    tok = m.get("token")
    if tok in ["0x06000466", "0x060004a7", "0x060004a9"] or m.get("name") in ["SetFeatureAura", "EnsureAuraStream"]:
        name = m.get("name")
        body = m.get("body", {})
        anchors = body.get("anchors", [])
        print(f"\n==================== {tok} : {name} ({len(anchors)} anchors) ====================")
        for a in anchors:
            op = a.get("opcode")
            val = a.get("operand", "")
            if op in ["ldsfld", "stsfld", "ldfld", "stfld"] and val in fields:
                val = f"{val} ({fields[val]})"
            print(f"  {a.get('il_offset'):04x}: {op} {val}")
