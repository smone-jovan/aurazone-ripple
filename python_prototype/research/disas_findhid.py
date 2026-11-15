from disas_il import disas

print("\n--- Disassembling FindHidStream (0x06000461) ---")
for line in disas(308672, 431, "fat"):
    print(line)
