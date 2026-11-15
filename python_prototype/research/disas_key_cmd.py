from disas_il import disas

print("\n--- Disassembling RunKeyCommand ---")
for line in disas(474124, 124, "fat"):
    print(line)

print("\n--- Disassembling CustomKey ---")
for line in disas(474276, 24, "fat"):
    print(line)
