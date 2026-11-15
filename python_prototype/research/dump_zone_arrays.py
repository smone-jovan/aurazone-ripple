import struct

with open(r"C:\Users\ROG\AuraZoneRipple\research\GHelper.dll", "rb") as f:
    dll = f.read()

# Let's search for the bytes in GHelper.dll
# We know the array length is 12 bytes.
# In live_test_direct.py:
# packet4ZoneFlipped: [0, 1, 2, 3, 0, 0, 4, 4, 5, 6, 7, 7]
# packet4Zone:        [0, 1, 2, 3, 0, 0, 7, 7, 6, 5, 4, 4]
pat1 = bytes([0, 1, 2, 3, 0, 0, 4, 4, 5, 6, 7, 7])
pat2 = bytes([0, 1, 2, 3, 0, 0, 7, 7, 6, 5, 4, 4])

print("pat1 found at:", dll.find(pat1))
print("pat2 found at:", dll.find(pat2))
