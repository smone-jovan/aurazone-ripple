import struct

# Standard CIL opcodes map
OPCODES = {
    0x00: ("nop", 0),
    0x01: ("break", 0),
    0x02: ("ldarg.0", 0),
    0x03: ("ldarg.1", 0),
    0x04: ("ldarg.2", 0),
    0x05: ("ldarg.3", 0),
    0x06: ("ldloc.0", 0),
    0x07: ("ldloc.1", 0),
    0x08: ("ldloc.2", 0),
    0x09: ("ldloc.3", 0),
    0x0a: ("stloc.0", 0),
    0x0b: ("stloc.1", 0),
    0x0c: ("stloc.2", 0),
    0x0d: ("stloc.3", 0),
    0x0e: ("ldarg.s", 1),
    0x0f: ("ldarga.s", 1),
    0x10: ("starg.s", 1),
    0x11: ("ldloc.s", 1),
    0x12: ("ldloca.s", 1),
    0x13: ("stloc.s", 1),
    0x14: ("ldnull", 0),
    0x15: ("ldc.i4.m1", 0),
    0x16: ("ldc.i4.0", 0),
    0x17: ("ldc.i4.1", 0),
    0x18: ("ldc.i4.2", 0),
    0x19: ("ldc.i4.3", 0),
    0x1a: ("ldc.i4.4", 0),
    0x1b: ("ldc.i4.5", 0),
    0x1c: ("ldc.i4.6", 0),
    0x1d: ("ldc.i4.7", 0),
    0x1e: ("ldc.i4.8", 0),
    0x1f: ("ldc.i4.s", 1), # int8
    0x20: ("ldc.i4", 4), # int32
    0x25: ("dup", 0),
    0x26: ("pop", 0),
    0x28: ("call", 4), # method token
    0x29: ("calli", 4),
    0x2a: ("ret", 0),
    0x2b: ("br.s", 1),
    0x2c: ("brfalse.s", 1),
    0x2d: ("brtrue.s", 1),
    0x2e: ("beq.s", 1),
    0x2f: ("bge.s", 1),
    0x30: ("bgt.s", 1),
    0x31: ("ble.s", 1),
    0x32: ("blt.s", 1),
    0x33: ("bne.un.s", 1),
    0x38: ("br", 4),
    0x39: ("brfalse", 4),
    0x3a: ("brtrue", 4),
    0x3b: ("beq", 4),
    0x3c: ("bge", 4),
    0x3d: ("bgt", 4),
    0x3e: ("ble", 4),
    0x3f: ("blt", 4),
    0x40: ("bne.un", 4),
    0x58: ("add", 0),
    0x59: ("sub", 0),
    0x5a: ("mul", 0),
    0x5b: ("div", 0),
    0x6f: ("callvirt", 4),
    0x72: ("ldstr", 4),
    0x73: ("newobj", 4),
    0x79: ("ldsfld", 4),
    0x7a: ("ldsflda", 4),
    0x7b: ("stsfld", 4),
    0x7c: ("ldfld", 4),
    0x7d: ("ldflda", 4),
    0x7e: ("stfld", 4),
    0x80: ("stsfld", 4),
    0x8d: ("newarr", 4),
    0x8e: ("ldlen", 0),
    0x8f: ("ldelema", 4),
    0x90: ("ldelem.i1", 0),
    0x91: ("ldelem.u1", 0),
    0x92: ("ldelem.i2", 0),
    0x93: ("ldelem.u2", 0),
    0x94: ("ldelem.i4", 0),
    0x95: ("ldelem.u4", 0),
    0x96: ("ldelem.i8", 0),
    0x97: ("ldelem.i", 0),
    0x98: ("ldelem.r4", 0),
    0x99: ("ldelem.r8", 0),
    0x9a: ("ldelem.ref", 0),
    0x9b: ("stelem.i", 0),
    0x9c: ("stelem.i1", 0),
    0x9d: ("stelem.i2", 0),
    0x9e: ("stelem.i4", 0),
    0x9f: ("stelem.i8", 0),
    0xa0: ("stelem.r4", 0),
    0xa1: ("stelem.r8", 0),
    0xa2: ("stelem.ref", 0),
    0xa3: ("ldelem", 4),
    0xa4: ("stelem", 4),
    0xd0: ("ldtoken", 4),
    0xd1: ("conv.u2", 0),
    0xd2: ("conv.u1", 0),
    0xd3: ("conv.i", 0),
    0xd6: ("conv.i4", 0),
}

with open(r"C:\Users\ROG\AuraZoneRipple\GHelper.dll", "rb") as f:
    dll = f.read()

def disas(file_offset, il_size, header_format="fat"):
    # If fat header, code starts 12 bytes after header
    ptr = file_offset + (12 if header_format == "fat" else 1)
    end = ptr + il_size
    il_offset = 0
    lines = []
    while ptr < end:
        op = dll[ptr]
        cur_il = il_offset
        ptr += 1
        il_offset += 1
        if op == 0xfe:
            op2 = dll[ptr]
            ptr += 1
            il_offset += 1
            name = f"FE_{op2:02x}"
            lines.append(f"{cur_il:04x}: {name}")
            continue
        if op in OPCODES:
            name, arg_len = OPCODES[op]
            if arg_len == 0:
                lines.append(f"{cur_il:04x}: {name}")
            elif arg_len == 1:
                arg = dll[ptr]
                if name.startswith("b") or name == "br.s":
                    # signed
                    s_arg = struct.unpack_from("<b", dll, ptr)[0]
                    target = il_offset + 1 + s_arg
                    lines.append(f"{cur_il:04x}: {name} {target:04x} (rel {s_arg:+d})")
                elif name == "ldc.i4.s":
                    s_arg = struct.unpack_from("<b", dll, ptr)[0]
                    lines.append(f"{cur_il:04x}: {name} {s_arg} (0x{arg:02x})")
                else:
                    lines.append(f"{cur_il:04x}: {name} {arg}")
                ptr += 1
                il_offset += 1
            elif arg_len == 4:
                val = struct.unpack_from("<I", dll, ptr)[0]
                if name.startswith("b") or name == "br":
                    s_val = struct.unpack_from("<i", dll, ptr)[0]
                    target = il_offset + 4 + s_val
                    lines.append(f"{cur_il:04x}: {name} {target:04x} (rel {s_val:+d})")
                elif name == "ldc.i4":
                    s_val = struct.unpack_from("<i", dll, ptr)[0]
                    lines.append(f"{cur_il:04x}: {name} {s_val} (0x{val:08x})")
                else:
                    lines.append(f"{cur_il:04x}: {name} 0x{val:08x}")
                ptr += 4
                il_offset += 4
        else:
            lines.append(f"{cur_il:04x}: db 0x{op:02x}")
    return lines

print("\n--- Disassembling ApplyDirect (0x060004a7) ---")
for line in disas(318404, 842):
    print(line)
