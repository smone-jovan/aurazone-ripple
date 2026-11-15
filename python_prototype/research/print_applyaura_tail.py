from disas_il import disas

lines = disas(319924, 846, "fat")
recording = False
for l in lines:
    if "02a0:" in l or "02b0:" in l or "02c0:" in l:
        recording = True
    if recording:
        print(l)
    if "0340:" in l:
        break
