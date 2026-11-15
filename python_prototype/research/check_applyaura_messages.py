from disas_il import disas

lines = disas(319924, 846, "fat")
for l in lines:
    if any(k in l for k in ["0x040005a5", "0x040005a6", "0x06000466"]):
        print(l)
