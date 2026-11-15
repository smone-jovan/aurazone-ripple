import json
import struct

with open(r"C:\Users\ROG\AuraZoneRipple\evidence.json", "r", encoding="utf-8") as f:
    data = json.load(f)

rec = [r for r in data["records"] if r.get("operation") == "inspect_managed_members"][0]
norm = rec.get("normalized_result", {})
fields = norm.get("fields", [])

# Let's find fields with RVA
with open(r"C:\Users\ROG\AuraZoneRipple\GHelper.dll", "rb") as f:
    dll_data = f.read()

# Parse PE sections
# e_lfanew at 0x3C
pe_offset = struct.unpack_from("<I", dll_data, 0x3C)[0]
num_sections = struct.unpack_from("<H", dll_data, pe_offset + 6)[0]
opt_header_size = struct.unpack_from("<H", dll_data, pe_offset + 20)[0]
sec_table = pe_offset + 24 + opt_header_size

sections = []
for i in range(num_sections):
    s_name = dll_data[sec_table + i*40 : sec_table + i*40 + 8].rstrip(b"\x00").decode("ascii", errors="ignore")
    vsize, vaddr, raw_size, raw_ptr = struct.unpack_from("<IIII", dll_data, sec_table + i*40 + 8)
    sections.append((s_name, vaddr, vsize, raw_ptr, raw_size))
    print(f"Section {s_name}: vaddr=0x{vaddr:X}, vsize=0x{vsize:X}, raw_ptr=0x{raw_ptr:X}, raw_size=0x{raw_size:X}")

def rva_to_offset(rva):
    for name, vaddr, vsize, raw_ptr, raw_size in sections:
        if vaddr <= rva < vaddr + max(vsize, raw_size):
            return raw_ptr + (rva - vaddr)
    return None

# Find field tokens in norm['fields']
target_tokens = ["0x04000aca", "0x04000b34", "0x04000b48", "0x04000b38", "0x04000aee", "0x04000ae4", "0x04000b2a"]
for f in fields:
    tok = f.get("token")
    if tok in target_tokens or any(name in f.get("name", "") for name in ["MAIN_AURA", "packet", "MESSAGE"]):
        rva = f.get("rva")
        print(f"Field: {f.get('name')} [token: {tok}], rva={rva}")
        if rva:
            file_off = rva_to_offset(rva)
            if file_off:
                raw_bytes = list(dll_data[file_off:file_off+32])
                print(f"  Offset: 0x{file_off:X}, bytes: {raw_bytes}")
