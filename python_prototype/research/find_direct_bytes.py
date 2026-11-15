import struct

with open(r"C:\Users\ROG\AuraZoneRipple\research\GHelper.dll", "rb") as f:
    dll = f.read()

# Let's search for calls and field loads in ApplyDirect (318404+12 .. 318404+12+842)
body = dll[318404+12 : 318404+12+842]

# Search for 0x5D, 0xBC
for i in range(len(body)-1):
    if body[i] == 0x5d and body[i+1] == 0xbc:
        print(f"Found 0x5D 0xBC at body offset {i}")
    if body[i] == 0xbc:
        print(f"Byte 0xBC at body offset {i}: {list(body[max(0, i-4):min(len(body), i+5)])}")
