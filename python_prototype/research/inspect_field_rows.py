import json

with open(r"C:\Users\ROG\AuraZoneRipple\evidence.json", "r", encoding="utf-8") as f:
    data = json.load(f)

rec = [r for r in data["records"] if r.get("operation") == "inspect_managed_members"][0]
norm = rec.get("normalized_result", {})
fields = norm.get("fields", [])

target_tokens = ["0x04000aca", "0x04000b34", "0x04000b48", "0x04000b38", "0x04000aee", "0x04000ae4", "0x04000b2a"]

for f in fields:
    tok = f.get("token")
    if tok in target_tokens or "AuraMode" in f.get("declaring_type", ""):
        print(f"{f.get('declaring_type')}::{f.get('name')} -> row_offset={f.get('row_offset')}, token={tok}")
