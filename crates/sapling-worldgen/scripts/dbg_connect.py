"""A bds_dbg.py probe (`"probe": "<this file>"`): walks BDS's block registry and writes <out dir>/connect.tsv, one
line per block type, what a template's update pass (141f5c5d0, docs/research/worldgen-bds-templates.md,
"The update pass") does with it:

  name  kind  vt_e0  vt_1b8  entries

`kind` is the pass's class for a placed block (141f5be10): 0 the type carries tag 0x14cd250d0 (every side
updated), 3 it has the neighbour-update component (id at 0x14cc3e300; updated on the sides the template did not
place), 2 its vtable slot 0xe0 is not the base one (`onStructureBlockPlace`, slot 0x1b8, on those sides), - none.
`vt_e0`/`vt_1b8` are those slots' functions (static VAs). `entries` lists the component's updaters as
`mask:update fn:setBlock flags` (mask bits: 1 down, 2 up, 4 north, 0x20 south, 0x100 west, 0x800 east, the rest
the up/down diagonals; the function is the updater's vtable slot 1). Registry layout as scripts/dbg_blocks.py.
"""
import struct

REGISTRY = 0x14CCB1608
COMPONENT_ID, COMPONENT_GUARD = 0x14CC3E300, 0x14CC3E304
ALL_SIDES_TAG = 0x14CD250D0
STATIC_BASE = 0x140000000


def probe(dbg, ctx):
    slide = dbg.base - STATIC_BASE

    def q(addr):
        data = dbg.mem(addr, 8)
        return struct.unpack("<Q", data)[0] if len(data) == 8 else 0

    def string(addr):
        size, cap = q(addr + 0x10), q(addr + 0x18)
        return dbg.mem(q(addr) if cap >= 0x10 else addr, min(size, 200)).decode("latin1")

    guard = struct.unpack("<i", dbg.mem(COMPONENT_GUARD + slide, 4))[0]
    component_id = struct.unpack("<H", dbg.mem(COMPONENT_ID + slide, 2))[0] if guard not in (0, -1) else None
    tag = q(ALL_SIDES_TAG + slide)

    def component(holder, ids_at):
        begin, end, values = q(holder + ids_at), q(holder + ids_at + 8), q(holder + ids_at + 0x18)
        count = (end - begin) // 2
        if component_id is None or not 0 < count < 4096:
            return None
        ids = struct.unpack(f"<{count}H", dbg.mem(begin, count * 2))
        return (q(values + 8 * ids.index(component_id)) or None) if component_id in ids else None

    def tagged(kind):
        head = q(kind + 0x1D8)
        stack = [q(head + 8)]
        while stack:
            node = stack.pop()
            if not node or node == head or dbg.mem(node + 0x19, 1) != b"\0":
                continue
            if q(node + 0x20) == tag:
                return True
            stack += [q(node), q(node + 0x10)]
        begin, end = q(kind + 0x320), q(kind + 0x328)
        return any(q(q(q(entry) + 8) + 8) == tag for entry in range(begin, min(end, begin + 16 * 64), 16))

    def entries(found):
        begin, end = q(found + 8), q(found + 0x10)
        out = []
        for entry in range(begin, min(end, begin + 0x18 * 32), 0x18):
            mask, updater, flags = struct.unpack("<H", dbg.mem(entry, 2))[0], q(entry + 8), dbg.mem(entry + 0x10, 1)[0]
            update = q(q(updater) + 8) - slide if updater else 0
            out.append(f"{mask:x}:{update:x}:{flags}")
        return ",".join(out)

    head = q(REGISTRY + slide)
    stack, rows, base_e0 = [q(head + 8)], [], {}
    while stack:
        node = stack.pop()
        if not node or dbg.mem(node + 0x19, 1) != b"\0":
            continue
        stack += [q(node), q(node + 0x10)]
        kind = q(node + 0x50)
        vtable, default = q(kind), q(kind + 0x240)
        found = (component(default, 8) if default else None) or component(kind, 0x30)
        e0, b8 = q(vtable + 0xE0) - slide, q(vtable + 0x1B8) - slide
        base_e0[e0] = base_e0.get(e0, 0) + 1
        rows.append([string(node + 0x28), tagged(kind), found, e0, b8])
    common = max(base_e0, key=base_e0.get)
    with open(f"{dbg.out}/connect.tsv", "w") as out:
        out.write(f"# component id {component_id}, tag {tag:x}, base slot 0xe0 {common:x}\n")
        for name, all_sides, found, e0, b8 in sorted(rows):
            kind = "0" if all_sides else "3" if found else "2" if e0 != common else "-"
            out.write(f"{name}\t{kind}\t{e0:x}\t{b8:x}\t{entries(found) if found else ''}\n")
    dbg.log.write(f"probe: {len(rows)} block types, component id {component_id}\n")
