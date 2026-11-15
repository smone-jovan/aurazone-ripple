import json
import struct

with open(r"C:\Users\ROG\AuraZoneRipple\research\evidence.json", "r", encoding="utf-8") as f:
    records = json.load(f)["records"]

rec = [r for r in records if r.get("operation") == "inspect_managed_members"][0]
fields = rec["normalized_result"]["fields"]
fld34 = [f for f in fields if f.get("token") == "0x04000b34"][0]
fld48 = [f for f in fields if f.get("token") == "0x04000b48"][0]
print("fld34:", fld34)
print("fld48:", fld48)

with open(r"C:\Users\ROG\AuraZoneRipple\research\GHelper.dll", "rb") as f:
    dll = f.read()

# Let's search in dll for references to 0x04000b34 or find FieldRVA table
