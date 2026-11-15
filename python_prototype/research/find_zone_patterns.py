with open(r"C:\Users\ROG\AuraZoneRipple\GHelper.dll", "rb") as f:
    dll = f.read()

# Let's search for candidate byte sequences of length 12
# We know 4-zone mappings on ASUS laptops are zone indices 0, 1, 2, 3 (and maybe lightbar 4, 5, 6, 7 or repetitions)
# All bytes must be in range 0..7!
# Let's search for sequences of 12 bytes where every byte is <= 7 and contains 0, 1, 2, 3.
candidates = []
for i in range(len(dll) - 12):
    chunk = dll[i:i+12]
    if all(b <= 7 for b in chunk) and all(z in chunk for z in [0, 1, 2, 3]):
        # Check if it has 0, 1, 2, 3
        candidates.append((i, list(chunk)))

print(f"Total candidates: {len(candidates)}")
for off, c in candidates[:20]:
    print(f"Offset 0x{off:X}: {c}")
