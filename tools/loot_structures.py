"""Writes `loot/row.mcstructure`: a row of seeded loot blocks for bds_loot_probe (tools/bds-loot-setup.sh).

usage: python3 tools/loot_structures.py <behavior pack dir>
Cells along +x (the rest structure void): 0, 2, 4 chests of simple_dungeon.json seed 357601146 (opened,
fed to a hopper, read by a comparator), 6 a decorated pot, 8 suspicious sand, 10 suspicious gravel.
"""
import os
import struct
import sys

VERSION = 18168865
DUNGEON = ("loot_tables/chests/simple_dungeon.json", 357601146)
POT = ("loot_tables/pots/trial_chambers/corridor.json", 1434522187)
SAND = ("loot_tables/entities/desert_pyramid_brushable_block.json", 1461741536)
GRAVEL = ("loot_tables/entities/trail_ruins_brushable_block_common.json", -1306338408)


class Tag:
    def __init__(self, kind, value, item=None):
        self.kind, self.value, self.item = kind, value, item


def byte(v): return Tag(1, v)
def short(v): return Tag(2, v)
def int_(v): return Tag(3, v)
def string(v): return Tag(8, v)
def compound(**fields): return Tag(10, fields)
def ints(values): return Tag(9, [int_(v) for v in values], 3)
def tags(kind, values): return Tag(9, values, kind)


def payload(tag):
    k, v = tag.kind, tag.value
    if k == 1:
        return struct.pack("<b", v)
    if k == 2:
        return struct.pack("<h", v)
    if k == 3:
        return struct.pack("<i", v)
    if k == 8:
        raw = v.encode()
        return struct.pack("<H", len(raw)) + raw
    if k == 9:
        return struct.pack("<bi", tag.item, len(v)) + b"".join(payload(t) for t in v)
    if k == 10:
        out = b""
        for name, t in v.items():
            raw = name.encode()
            out += struct.pack("<bH", t.kind, len(raw)) + raw + payload(t)
        return out + b"\0"
    raise ValueError(k)


def block(name, **states):
    return compound(name=string("minecraft:" + name), states=compound(**states), version=int_(VERSION))


def loot(entity_id, table, seed, **extra):
    return compound(block_entity_data=compound(id=string(entity_id), LootTable=string(table), LootTableSeed=int_(seed), x=int_(0), y=int_(0), z=int_(0), **extra))


def main():
    pack = sys.argv[1]
    empty_item = compound(Count=byte(0), Damage=short(0), Name=string(""), WasPickedUp=byte(0))
    palette = [
        block("structure_void", structure_void_type=string("void")),
        block("chest", facing_direction=int_(2)),
        block("decorated_pot", direction=int_(0)),
        block("suspicious_sand", brushed_progress=int_(0), hanging=byte(1)),
        block("suspicious_gravel", brushed_progress=int_(0), hanging=byte(1)),
    ]
    cells = {0: 1, 2: 1, 4: 1, 6: 2, 8: 3, 10: 4}
    data = {
        0: loot("Chest", *DUNGEON, Findable=byte(0), Items=tags(10, [])),
        2: loot("Chest", *DUNGEON, Findable=byte(0), Items=tags(10, [])),
        4: loot("Chest", *DUNGEON, Findable=byte(0), Items=tags(10, [])),
        6: loot("DecoratedPot", *POT, animation=byte(0), item=empty_item),
        8: loot("BrushableBlock", *SAND, brush_count=int_(0), brush_direction=byte(6), type=string("minecraft:suspicious_sand")),
        10: loot("BrushableBlock", *GRAVEL, brush_count=int_(0), brush_direction=byte(6), type=string("minecraft:suspicious_gravel")),
    }
    width = 11
    # Index order is x, then y, then z (z fastest); the size is 1 high and 1 deep.
    layer = [cells.get(x, 0) for x in range(width)]
    root = compound(
        format_version=int_(1),
        size=ints([width, 1, 1]),
        structure=compound(
            block_indices=tags(9, [ints(layer), ints([-1] * width)]),
            entities=tags(10, []),
            palette=compound(default=compound(block_palette=tags(10, palette), block_position_data=compound(**{str(i): t for i, t in data.items()}))),
        ),
        structure_world_origin=ints([0, 0, 0]),
    )
    os.makedirs(os.path.join(pack, "structures", "loot"), exist_ok=True)
    with open(os.path.join(pack, "structures", "loot", "row.mcstructure"), "wb") as f:
        f.write(struct.pack("<bH", 10, 0) + payload(root))


main()
