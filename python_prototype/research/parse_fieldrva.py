import struct

with open(r"C:\Users\ROG\AuraZoneRipple\GHelper.dll", "rb") as f:
    data = f.read()

# PE Header
pe_offset = struct.unpack_from("<I", data, 0x3C)[0]
opt_header_size = struct.unpack_from("<H", data, pe_offset + 20)[0]
num_sections = struct.unpack_from("<H", data, pe_offset + 6)[0]
sec_table = pe_offset + 24 + opt_header_size

sections = []
for i in range(num_sections):
    s_name = data[sec_table + i*40 : sec_table + i*40 + 8].rstrip(b"\x00").decode("ascii", errors="ignore")
    vsize, vaddr, raw_size, raw_ptr = struct.unpack_from("<IIII", data, sec_table + i*40 + 8)
    sections.append((s_name, vaddr, vsize, raw_ptr, raw_size))

def rva_to_offset(rva):
    for name, vaddr, vsize, raw_ptr, raw_size in sections:
        if vaddr <= rva < vaddr + max(vsize, raw_size):
            return raw_ptr + (rva - vaddr)
    return None

# CLI Header is at data directory 14 (offset 0xE8 in PE32+)
cli_rva = struct.unpack_from("<I", data, pe_offset + 24 + 208)[0]
cli_off = rva_to_offset(cli_rva)
meta_rva = struct.unpack_from("<I", data, cli_off + 8)[0]
meta_off = rva_to_offset(meta_rva)

# Metadata root
sig = data[meta_off:meta_off+4]
v_len = struct.unpack_from("<I", data, meta_off + 12)[0]
streams_off = meta_off + 16 + v_len
streams_off = (streams_off + 3) & ~3
num_streams = struct.unpack_from("<H", data, streams_off + 2)[0]

stream_headers = []
p = streams_off + 4
for i in range(num_streams):
    s_off, s_size = struct.unpack_from("<II", data, p)
    p += 8
    name = []
    while data[p] != 0:
        name.append(chr(data[p]))
        p += 1
    p += 1
    p = (p + 3) & ~3
    stream_headers.append(("".join(name), meta_off + s_off, s_size))

print("Streams:", stream_headers)

# Find ~ stream (table stream)
t_off = [s[1] for s in stream_headers if s[0] == "#~"][0]
valid_mask = struct.unpack_from("<Q", data, t_off + 8)[0]
rows = {}
p = t_off + 24
for i in range(64):
    if valid_mask & (1 << i):
        count = struct.unpack_from("<I", data, p)[0]
        rows[i] = count
        p += 4

# Field index size: if rows.get(4, 0) > 65535: 4 else 2
field_idx_size = 4 if rows.get(4, 0) > 65535 else 2

# Calculate offset to table 0x1D (FieldRVA)
# Tables before 0x1D: 0..0x1C
# We need row sizes of each table before 0x1D
# Or simply search for FieldRVA table rows!
