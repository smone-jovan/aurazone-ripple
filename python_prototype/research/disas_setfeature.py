from disas_il import disas

print("\n--- Disassembling SetFeatureAura (0x06000466) ---")
for line in disas(310500, 182, "fat"):
    print(line)
