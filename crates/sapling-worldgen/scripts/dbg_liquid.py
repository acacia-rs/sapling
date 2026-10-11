"""A bds_dbg.py probe (`"probe": "<this file>"`): walks BDS's block registry and writes <out dir>/liquid.tsv, one
line per block type and distinct liquid detection among its permutations:

  name  permutations  can_contain  stops_from  on_touch  material  [state indices when the type is not uniform]

The three bytes are `Block` +0xb8, +0xb9 and +0xba, what `LiquidBlock::_isLiquidBlocking` (148b5ee50) reads:
`on_touch` 2 holds water out whatever the side, otherwise the side's bit in `stops_from` decides (table
14a8548e8) before the type's own test. `material` is the type's material id (lava asks the type instead).
Registry layout as scripts/dbg_blocks.py; a type's permutations are the `Block*` vector at +0x228.
scripts/liquid_data.py turns the file (kept as scripts/data/block_liquid.tsv) into feature/liquid/detect_table.rs.
"""
import struct

REGISTRY = 0x14CCB1608
STATIC_BASE = 0x140000000


def probe(dbg, ctx):
    slide = dbg.base - STATIC_BASE

    def q(addr):
        data = dbg.mem(addr, 8)
        return struct.unpack("<Q", data)[0] if len(data) == 8 else 0

    def string(addr):
        size, cap = q(addr + 0x10), q(addr + 0x18)
        return dbg.mem(q(addr) if cap >= 0x10 else addr, min(size, 200)).decode("latin1")

    head = q(REGISTRY + slide)
    stack, rows = [q(head + 8)], []
    while stack:
        node = stack.pop()
        if not node or dbg.mem(node + 0x19, 1) != b"\0":
            continue
        stack += [q(node), q(node + 0x10)]
        kind = q(node + 0x50)
        begin, end = q(kind + 0x228), q(kind + 0x230)
        count = (end - begin) // 8
        blocks = struct.unpack(f"<{count}Q", dbg.mem(begin, count * 8)) if 0 < count < 70000 else ()
        material = dbg.mem(q(kind + 0x28), 1)
        seen = {}
        for index, block in enumerate(blocks):
            if block:
                seen.setdefault(tuple(dbg.mem(block + 0xB8, 3)), []).append(index)
        for fields, indices in sorted(seen.items()):
            which = "" if len(seen) == 1 else ",".join(map(str, indices))
            rows.append((string(node + 0x28), count, *fields, material[0] if material else -1, which))
    with open(f"{dbg.out}/liquid.tsv", "w") as out:
        for row in sorted(rows):
            out.write("\t".join(map(str, row)).rstrip() + "\n")
    dbg.log.write(f"probe: {len(rows)} liquid detection rows\n")
