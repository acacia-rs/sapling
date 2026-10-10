"""A bds_dbg.py probe (`"probe": "<this file>"` on any breakpoint hit once the world generates): walks BDS's block
registry and writes <out dir>/blocks.tsv, one line per block type:

  name  vtable  canProvideSupport  canProvideMultifaceSupport  properties  material  blocksMotion  solid  support

`canProvideSupport` and `canProvideMultifaceSupport` are the functions in the type's vtable slots 0x90 and 0x98
(static VAs), `properties` the u64 at `BlockType` +0x130 (bit 0x40000 solid-blocking, 0x20000 top support), the
material fields its type and its +3 and +5 bytes, `support` the kind in the `minecraft:support` component of the
type's default block (0 fence, 1 stair; - none). scripts/support_data.py turns the file into a Rust table.
Layouts (image 6ab54bee): registry 0x14ccb1608 holds an MSVC std::map head; a node has left, parent, right, the
is-nil byte at +0x19, the name hash at +0x20, its string at +0x28 and the `BlockType*` at +0x50.
"""
import struct

REGISTRY = 0x14CCB1608
SUPPORT_ID, SUPPORT_GUARD = 0x14CE19564, 0x14CE19568
STATIC_BASE = 0x140000000


def probe(dbg, ctx):
    slide = dbg.base - STATIC_BASE

    def q(addr):
        data = dbg.mem(addr, 8)
        return struct.unpack("<Q", data)[0] if len(data) == 8 else 0

    def string(addr):
        size, cap = q(addr + 0x10), q(addr + 0x18)
        return dbg.mem(q(addr) if cap >= 0x10 else addr, min(size, 200)).decode("latin1")

    guard = struct.unpack("<i", dbg.mem(SUPPORT_GUARD + slide, 4))[0]
    support_id = struct.unpack("<H", dbg.mem(SUPPORT_ID + slide, 2))[0] if guard not in (0, -1) else None

    def component(holder, ids_at):
        """The component with the support id in a sorted (ids, pointers) pair of vectors."""
        begin, end, values = q(holder + ids_at), q(holder + ids_at + 8), q(holder + ids_at + 0x18)
        count = (end - begin) // 2
        if support_id is None or not 0 < count < 4096:
            return None
        ids = struct.unpack(f"<{count}H", dbg.mem(begin, count * 2))
        if support_id not in ids:
            return None
        return q(values + 8 * ids.index(support_id)) or None

    head = q(REGISTRY + slide)
    stack, rows = [q(head + 8)], []
    while stack:
        node = stack.pop()
        if not node or dbg.mem(node + 0x19, 1) != b"\0":
            continue
        stack += [q(node), q(node + 0x10)]
        kind = q(node + 0x50)
        vtable, material, default = q(kind), q(kind + 0x28), q(kind + 0x240)
        found = (component(default, 8) if default else None) or component(kind, 0x30)
        support = struct.unpack("<i", dbg.mem(found + 8, 4))[0] if found else "-"
        fields = dbg.mem(material, 6)
        rows.append((string(node + 0x28), f"{vtable - slide:x}", f"{q(vtable + 0x90) - slide:x}", f"{q(vtable + 0x98) - slide:x}",
                     f"{q(kind + 0x130):x}", fields[0], fields[3], fields[5], support))
    with open(f"{dbg.out}/blocks.tsv", "w") as out:
        out.write(f"# support component id {support_id}\n")
        for row in sorted(rows):
            out.write("\t".join(map(str, row)) + "\n")
    dbg.log.write(f"probe: {len(rows)} block types\n")
