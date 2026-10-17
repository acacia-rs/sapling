"""A bds_dbg.py probe (`"probe": "<this file>"`): writes <out dir>/block_ptrs.tsv, `pointer name permutation` per line
for every permutation of every block type, so dumped `Block*` buffers (scripts/data/dbg_nether_buffers.json) read as
names. Registry layout as scripts/dbg_blocks.py; a type's permutations are the `Block*` vector at +0x228.
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
        name = string(node + 0x28)
        rows += [(block, name, index) for index, block in enumerate(blocks) if block]
    with open(f"{dbg.out}/block_ptrs.tsv", "w") as out:
        for block, name, index in rows:
            out.write(f"{block:x}\t{name}\t{index}\n")
    dbg.log.write(f"probe: {len(rows)} block pointers\n")
