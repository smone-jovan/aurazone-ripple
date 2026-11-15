from disas_il import disas

print("\n--- Disassembling FindDevices MoveNext ---")
for line in disas(566564, 465, "fat"):
    print(line)
