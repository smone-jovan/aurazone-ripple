import hashlib

with open(r"C:\Users\ROG\AuraZoneRipple\GHelper.dll", "rb") as f:
    dll_data = f.read()

targets = {
    "MAIN_AURA_PIDS (100b)": (100, "47245F2C58E6314C67E8919E977284E92B5FB39528D0F485412561D336799CBE".lower()),
    "initDirect (b2a)": (16, "ca4c78386d3f71648355e36cc126d418b59098b60c6766d9d7d3eaf479025ab7".lower()),
    "packet4Zone (12b)": (12, "D79D811DC868901EEF42FABC16231A692CB7D1FB53B73D9B38D5AB8B957DB61F".lower()),
    "packet4ZoneFlipped (12b)": (12, "F15DD7038C36F6DC06B4633136EA05E9D60DB88E8B586E075863D634E4A38916".lower()),
    "packetZone (135b)": (135, "7632BCD9F221DAE4870DE732B559EBE06E3061BE1DEA892F2FD4427C24B8004D".lower()),
    "packetZoneNumpad (135b)": (135, "6F1AFAB5E2A790B2618CD04360FC6962BD066BD5713CC9C618FA39B066541F10".lower()),
    "packetMap (135b)": (135, "DD8C9219495223FCFCB5C2DA9AF38745E3F5D4ED9009A9E1AF8360DC939C1CC1".lower()),
}

for name, (size, target_hash) in targets.items():
    found = False
    for i in range(len(dll_data) - size):
        chunk = dll_data[i:i+size]
        if hashlib.sha256(chunk).hexdigest() == target_hash:
            print(f"FOUND {name} at offset 0x{i:X}:")
            print(f"  hex: {chunk.hex()}")
            print(f"  list: {list(chunk)}")
            found = True
            break
    if not found:
        # Maybe size was slightly different? Search variable sizes around it
        for s in range(max(1, size-4), size+10):
            for i in range(len(dll_data) - s):
                chunk = dll_data[i:i+s]
                if hashlib.sha256(chunk).hexdigest() == target_hash:
                    print(f"FOUND {name} (size {s}) at offset 0x{i:X}:")
                    print(f"  hex: {chunk.hex()}")
                    print(f"  list: {list(chunk)}")
                    found = True
                    break
            if found: break
    if not found:
        print(f"NOT FOUND: {name}")
