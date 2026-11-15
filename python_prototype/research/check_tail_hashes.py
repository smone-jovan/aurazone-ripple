import hashlib

with open(r"C:\Users\ROG\AuraZoneRipple\GHelper.dll", "rb") as f:
    dll = f.read()

# Let's inspect data between 0x184000 and 0x186000
# And match hashes!
targets = {
    "MAIN_AURA_PIDS": "47245F2C58E6314C67E8919E977284E92B5FB39528D0F485412561D336799CBE".lower(),
    "initDirect": "ca4c78386d3f71648355e36cc126d418b59098b60c6766d9d7d3eaf479025ab7".lower(),
    "packet4Zone": "D79D811DC868901EEF42FABC16231A692CB7D1FB53B73D9B38D5AB8B957DB61F".lower(),
    "packet4ZoneFlipped": "F15DD7038C36F6DC06B4633136EA05E9D60DB88E8B586E075863D634E4A38916".lower(),
    "packetZone": "7632BCD9F221DAE4870DE732B559EBE06E3061BE1DEA892F2FD4427C24B8004D".lower(),
    "packetZoneNumpad": "6F1AFAB5E2A790B2618CD04360FC6962BD066BD5713CC9C618FA39B066541F10".lower(),
    "packetMap": "DD8C9219495223FCFCB5C2DA9AF38745E3F5D4ED9009A9E1AF8360DC939C1CC1".lower(),
}

start = 0x184000
end = 0x186000

for off in range(start, end):
    for size in [3, 4, 8, 12, 16, 20, 24, 25, 48, 64, 100, 135, 136, 144, 200, 256]:
        if off + size <= len(dll):
            chunk = dll[off:off+size]
            h = hashlib.sha256(chunk).hexdigest()
            for name, thash in targets.items():
                if h == thash:
                    print(f"MATCH! {name} at 0x{off:X} (size {size}):")
                    print(f"  {list(chunk)}")
