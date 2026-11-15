import json

with open(r"C:\Users\ROG\AuraZoneRipple\evidence.json", "r", encoding="utf-8") as f:
    data = json.load(f)

rec = [r for r in data["records"] if r.get("operation") == "inspect_managed_members"][0]
norm = rec.get("normalized_result", {})
fields = norm.get("fields", [])

# Let's inspect the types to find AuraMode and its fields
for f in fields:
    if f.get("declaring_type") == "GHelper.USB.AuraMode":
        name = f.get("name")
        tok = f.get("token")
        # In evidence, check if raw_result or other info has constant value
        print(f"AuraMode::{name} [token: {tok}] -> {f.get('constant')}")
