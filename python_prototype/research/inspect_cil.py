import json

with open(r"C:\Users\ROG\AuraZoneRipple\evidence.json", "r", encoding="utf-8") as f:
    data = json.load(f)

rec = [r for r in data["records"] if r.get("operation") == "inspect_managed_members"][0]
norm = rec.get("normalized_result", {})
methods = norm.get("methods", [])

tokens_to_inspect = {
    "0x0600045e": "EnsureAuraStream",
    "0x06000460": "FindDevices",
    "0x06000461": "FindHidStream",
    "0x06000466": "SetFeatureAura",
    "0x0600049e": "DetectBacklightType",
    "0x060004a7": "ApplyDirect_1",
    "0x060004a8": "ApplyDirectLightbar",
    "0x060004a9": "ApplyDirect_2",
    "0x060004ac": "ApplyAura",
}

for m in methods:
    tok = m.get("token")
    if tok in tokens_to_inspect:
        name = tokens_to_inspect[tok]
        body = m.get("body", {})
        print(f"\n==================== {tok} : {name} ====================")
        print("Body info:", {k: v for k, v in body.items() if k != "instructions"})
        instrs = body.get("instructions", [])
        print(f"Instruction count: {len(instrs)}")
        for idx, ins in enumerate(instrs):
            print(f"  {ins.get('offset', idx):04x}: {ins.get('opcode')} {ins.get('operand', '')}")
