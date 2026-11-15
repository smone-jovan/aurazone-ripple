from disas_il import disas

with open(r"C:\Users\ROG\AuraZoneRipple\GHelper.dll", "rb") as f:
    dll = f.read()

import json
with open(r"C:\Users\ROG\AuraZoneRipple\evidence.json", "r", encoding="utf-8") as f:
    data = json.load(f)

rec = [r for r in data["records"] if r.get("operation") == "inspect_managed_members"][0]
norm = rec.get("normalized_result", {})
methods = norm.get("methods", [])

# Let's inspect all methods in GHelper.USB.Aura
for m in methods:
    decl = m.get("declaring_type", "")
    if decl == "GHelper.USB.Aura":
        b = m.get("body", {})
        fo = b.get("file_offset")
        sz = b.get("il_size")
        fmt = b.get("header_format", "fat")
        if fo and sz:
            lines = disas(fo, sz, fmt)
            # check if 24 or 0x18 is in lines
            for l in lines:
                if "24" in l or "0x18" in l.lower():
                    print(f"Method {m.get('name')}: {l}")
