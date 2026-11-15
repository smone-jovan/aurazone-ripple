from disas_il import disas

print("\n--- Disassembling Timer_Elapsed ---")
for line in disas(315570, 55, "tiny"):
    print(line)
