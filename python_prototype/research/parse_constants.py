import struct

with open(r"C:\Users\ROG\AuraZoneRipple\GHelper.dll", "rb") as f:
    dll = f.read()

# Find `#~` stream
pos = 0
tables_offset = dll.find(b"#~\x00\x00")
print("Found #~ at:", tables_offset)
if tables_offset != -1:
    # 4 bytes before #~ is offset from metadata root, size is 4 bytes before that
    s_offset, s_size = struct.unpack_from("<II", dll, tables_offset - 8)
    meta_root = tables_offset - 8 - s_offset
    print(f"Metadata root at 0x{meta_root:X}, #~ at 0x{meta_root + s_offset:X}")
    
    t_base = meta_root + s_offset
    valid_mask = struct.unpack_from("<Q", dll, t_base + 8)[0]
    
    ptr = t_base + 24
    row_counts = {}
    for i in range(64):
        if valid_mask & (1 << i):
            cnt, = struct.unpack_from("<I", dll, ptr)
            row_counts[i] = cnt
            ptr += 4
    
    # Blob stream
    blob_pos = dll.find(b"#Blob\x00\x00\x00")
    b_offset, b_size = struct.unpack_from("<II", dll, blob_pos - 8)
    blob_base = meta_root + b_offset
    print(f"#Blob at 0x{blob_base:X}")

    # Field count: row_counts[4]
    # Constant count: row_counts[11]
    # HasConstant coded index: Field is tag 0 (2 bits).
    # Since Field count is row_counts[4] (4194 < 16384), Parent is 2 bytes!
    # Blob index size: 2 or 4? (Check heap sizes byte at t_base + 6)
    heap_sizes = dll[t_base + 6]
    blob_idx_size = 4 if (heap_sizes & 0x04) else 2
    parent_idx_size = 2 if (max(row_counts.get(4,0), row_counts.get(8,0), row_counts.get(23,0)) < 16384) else 4
    
    print(f"Parent size: {parent_idx_size}, Blob idx size: {blob_idx_size}")
    
    # Calculate offset of table 11 (Constant)
    # We must calculate size of tables 0..10
    # Or search for Field token 0x04000588 in Constant table!
    # Field 0x04000588 is field index 0x588 = 1416.
    # HasConstant coded index for Field with index 1416 is (1416 << 2) | 0 = 5664 (0x1620)
    target_coded = (1416 << 2) | 0
    target_bytes = struct.pack("<H", target_coded)
    print(f"Looking for coded index 0x{target_coded:X} ({target_bytes.hex()})")
    
    # Search in #~ for target_bytes
    idx = 0
    while True:
        pos = dll.find(target_bytes, idx)
        if pos == -1: break
        # Check if type byte 2 bytes before is 0x08 (I4)
        if dll[pos-2] == 0x08 and dll[pos-1] == 0x00:
            print(f"Candidate Constant row at 0x{pos-2:X}")
            blob_idx = struct.unpack_from("<H" if blob_idx_size == 2 else "<I", dll, pos+2)[0]
            # Read blob at blob_base + blob_idx
            # First byte is length
            blen = dll[blob_base + blob_idx]
            bval = dll[blob_base + blob_idx + 1 : blob_base + blob_idx + 1 + blen]
            ival, = struct.unpack("<i", bval) if len(bval) == 4 else (bval,)
            print(f"  Field 0x04000588 constant value = {ival}")
            break
        idx = pos + 1
