from disas_il import disas

print("\n--- Disassembling EnsureAuraStream (0x0600045e) ---")
for line in disas(308524, 75, "fat"):
    print(line)
