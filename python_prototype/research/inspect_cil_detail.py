import json

with open(r"C:\Users\ROG\AuraZoneRipple\evidence.json", "r", encoding="utf-8") as f:
    data = json.load(f)

rec = [r for r in data["records"] if r.get("operation") == "inspect_managed_members"][0]
norm = rec.get("normalized_result", {})
methods = norm.get("methods", [])
user_strings = norm.get("user_strings", {})

tokens_to_inspect = {
    "0x0600045e": "EnsureAuraStream",
    "0x06000460": "FindDevices",
    "0x06000461": "FindHidStream",
    "0x06000464": "Write_1",
    "0x06000465": "Write_2",
    "0x06000466": "SetFeatureAura",
    "0x0600049e": "DetectBacklightType",
    "0x060004a2": "DirectBrightness",
    "0x060004a7": "ApplyDirect_1",
    "0x060004a8": "ApplyDirectLightbar",
    "0x060004a9": "ApplyDirect_2",
}

for m in methods:
    tok = m.get("token")
    if tok in tokens_to_inspect:
        name = tokens_to_inspect[tok]
        body = m.get("body", {})
        anchors = body.get("anchors", [])
        print(f"\n==================== {tok} : {name} ({len(anchors)} anchors) ====================")
        for a in anchors:
            op = a.get("opcode")
            val = a.get("operand", "")
            if op == "ldstr" and val in user_strings:
                val = f"{val} -> \"{user_strings[val].get('value', '')}\""
            print(f"  {a.get('il_offset'):04x}: {op} {val}")
