import struct

with open(r"C:\Users\ROG\AuraZoneRipple\GHelper.dll", "rb") as f:
    data = f.read()

# Metadata offset: RVA 597780 -> File offset 592880
meta_off = 592880

v_len = struct.unpack_from("<I", data, meta_off + 12)[0]
p = (meta_off + 16 + v_len + 3) & ~3
num_streams = struct.unpack_from("<H", data, p + 2)[0]
p += 4
streams = {}
for i in range(num_streams):
    s_off, s_size = struct.unpack_from("<II", data, p)
    p += 8
    name = []
    while data[p] != 0:
        name.append(chr(data[p]))
        p += 1
    p = (p + 1 + 3) & ~3
    streams["".join(name)] = (meta_off + s_off, s_size)

print("Streams:", {k: hex(v[0]) for k, v in streams.items()})

blob_base = streams["#Blob"][0]
table_base = streams["#~"][0]

# Parse #~
valid_mask = struct.unpack_from("<Q", data, table_base + 8)[0]
rows = {}
ptr = table_base + 24
for i in range(64):
    if valid_mask & (1 << i):
        count = struct.unpack_from("<I", data, ptr)[0]
        rows[i] = count
        ptr += 4

print("Rows:", {k: rows[k] for k in sorted(rows.keys()) if k in [1, 2, 4, 6, 8, 11, 29]})
