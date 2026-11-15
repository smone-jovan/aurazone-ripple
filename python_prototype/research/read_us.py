with open(r"C:\Users\ROG\AuraZoneRipple\GHelper.dll", "rb") as f:
    dll = f.read()

# Find #US stream
us_pos = dll.find(b"#US\x00")
import struct
s_offset, s_size = struct.unpack_from("<II", dll, us_pos - 8)
meta_root = us_pos - 8 - s_offset
us_base = meta_root + s_offset

for tok in [0x0000d248, 0x0000cebc, 0x0000cee8]:
    p = us_base + tok
    length = dll[p] # first byte or 7bit int
    # UTF-16LE
    s = dll[p+1 : p+length].decode("utf-16le", errors="ignore")
    print(f"Token 0x70{tok:06X}: {repr(s)}")
