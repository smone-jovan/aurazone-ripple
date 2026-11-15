import json

with open(r"C:\Users\ROG\AuraZoneRipple\evidence.json", "r", encoding="utf-8") as f:
    records = json.load(f)["records"]

for r in records:
    norm = r.get("normalized_result", {})
    types = norm.get("types", [])
    for t in types:
        if "aura" in t.get("name", "").lower():
            print("Type:", t.get("name"), "Methods:", len(t.get("methods", [])))
            for m in t.get("methods", []):
                print("  Method:", m.get("name"), m.get("signature"))
