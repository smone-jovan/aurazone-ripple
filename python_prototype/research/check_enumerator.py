import json

with open(r"C:\Users\ROG\AuraZoneRipple\evidence.json", "r", encoding="utf-8") as f:
    data = json.load(f)

rec = [r for r in data["records"] if r.get("operation") == "inspect_managed_members"][0]
norm = rec.get("normalized_result", {})
methods = norm.get("methods", [])

m_iter = [m for m in methods if m.get("token") == "0x06001247"][0]
decl = m_iter.get("declaring_type")
print("Enumerator declaring type:", decl)

# Find MoveNext of that type
move_next = [m for m in methods if m.get("declaring_type") == decl and m.get("name") == "MoveNext"][0]
print("MoveNext body offset:", move_next.get("body", {}).get("file_offset"), "size:", move_next.get("body", {}).get("il_size"))
