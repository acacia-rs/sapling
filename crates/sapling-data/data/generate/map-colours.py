"""BDS map captures -> map-colours.tsv: each block type's map colour and tint, read off the maps BDS drew
of the map-colours pack's grids (tools/map-colours). See ../../README.md "Map colours".

usage: py -3 map-colours.py
"""
import os
import re

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
CAPTURES = os.path.join(HERE, "..", "..", "..", "..", "research", "captures")
OUT = os.path.join(HERE, "..", "map-colours.tsv")

f32 = np.float32
NORMAL = f32(0.8627451)
GRASS_NORMAL, DIRT_LOW, GRASS_LOW = "7da24b", "6a4c36", "67843e"
# What a raised cell shows when it holds nothing of its own: ground, or the water a neighbour let out.
SPOILT = ("?", GRASS_NORMAL, GRASS_LOW, DIRT_LOW, "91bc58", "1e5af5")
# Java's MapColor list: a block whose raised cell was spoilt is matched against it by its normal pixel.
JAVA = """000000 7fb238 f7e9a3 c7c7c7 ff0000 a0a0ff a7a7a7 007c00 ffffff a4a8b8 976d4d 707070 4040ff 8f7748 fffcf5 d87f33
b24cd8 6699d8 e5e533 7fcc19 f27fa5 4c4c4c 999999 4c7f99 7f3fb2 334cb2 664c33 667f33 993333 191919 faee4d 5cdbd5 4a80ff 00d93a
815631 700200 d1b1a1 9f5224 95576c 706c8a ba8524 677535 a04d4e 392923 876b62 575c5c 7a4958 4c3e5c 4c3223 4c522a 8e3c2e 251610
bd3031 943f61 5c191d 167e86 3a8e8c 562c3e 14b485 646464 d8af93 7fa796""".split()
# Blocks whose grid cell said nothing (they broke or melted before the map was drawn): Java's colour,
# or a block of the same kind.
BY_HAND = {
    "minecraft:cactus": "007c00",
    "minecraft:frosted_ice": "a0a0ff",
    "minecraft:jungle_wall_sign": "=minecraft:jungle_standing_sign",
    "minecraft:underwater_torch": "-",
}
TINTS = {"None": "-", "Grass": "grass", "DefaultFoliage": "foliage", "BirchFoliage": "birch", "EvergreenFoliage": "evergreen", "Water": "water"}


def shade(channel, by):
    return int(f32(f32(f32(channel) / f32(255)) * by) * f32(255))


def rgb(pixel):
    return tuple(int(pixel[i:i + 2], 16) for i in (0, 2, 4))


def normal(pixel):
    return "".join(f"{shade(v, NORMAL):02x}" for v in rgb(pixel))


def pixels(capture):
    """The pixels of the map drawn last and whole in a capture, by (column, row), as rrggbb."""
    maps, current = {}, None
    for line in open(capture, encoding="utf-8"):
        head = re.search(r"MapPixels map=(-?\d+) x=(\d+) y=(\d+) w=(\d+) h=(\d+)", line)
        if head:
            current = [int(g) for g in head.groups()]
            continue
        row = re.search(r"MapPixels row (\d+) (.*)", line)
        if row and current:
            map_id, x0, y0, _, _ = current
            drawn, column = maps.setdefault(map_id, {}), 0
            for run in row.group(2).split():
                count, value = run.split("*")
                for _ in range(int(count)):
                    drawn[(x0 + column, y0 + int(row.group(1)))] = value[:6]
                    column += 1
    return max(maps.values(), key=len)


def cells(path):
    """Each grid cell's block: name -> (x, z, map_color component or None)."""
    found = {}
    for line in open(path, encoding="utf-8"):
        cell = re.match(r"MAPCOL cell (-?\d+) (-?\d+) (\S+)(?: comp=(\S+))?", line)
        if cell:
            found[cell.group(3)] = (int(cell.group(1)), int(cell.group(2)), cell.group(4))
    return found


def seen(grid):
    """Each block's own pixel and its north neighbour's on the map of a grid (centred on 0, 0)."""
    drawn, laid = pixels(os.path.join(CAPTURES, f"bds-{grid}.txt")), cells(os.path.join(CAPTURES, f"bds-{grid}.cells"))
    at = lambda x, z: drawn.get((x + 64, z + 64), "?")
    return {name: (at(x, z), at(x, z - 1)) for name, (x, z, _) in laid.items()}, laid


def between(raised, ground):
    """A colour between whole bytes: the middle of what the full and the normal pixel both allow."""
    parts = []
    for full, shaded in zip(rgb(raised), rgb(ground)):
        low, high = max(full, shaded / float(NORMAL)), min(full + 1, (shaded + 1) / float(NORMAL))
        if low >= high:
            return None
        parts.append(f"{round((low + high) / 2, 2):g}")
    return ",".join(parts)


def main():
    ground, laid = seen("mapgrid")
    raised, raised_laid = seen("mapgrid-raised")
    palette = set(JAVA) | {raised[name][0] for name in ground if name in raised and normal(raised[name][0]) == ground[name][0]}
    rows = {}
    for name in sorted(ground):
        pixel, north = ground[name]
        full = raised.get(name, ("?", "?"))[0]
        component = raised_laid.get(name, (0, 0, None))[2] or laid[name][2]
        if component:
            colour, tint, _ = component.split("|")
            channels = [round(float(v)) for v in colour.split(",")[:3]]
            rows[name] = ("".join(f"{v:02x}" for v in channels), TINTS[tint], "component")
        elif name in BY_HAND:
            rows[name] = (BY_HAND[name], "-", "by hand")
        elif pixel == GRASS_LOW and full not in SPOILT:
            # Gone from its ground cell (grass grew into the dirt it left), drawn in its raised one.
            rows[name] = (full, "-", "raised")
        elif pixel in (DIRT_LOW, GRASS_LOW):
            rows[name] = ("-", "-", "none")
        elif north == GRASS_NORMAL and normal(full) == pixel:
            rows[name] = (full, "-", "raised")
        elif len(matches := sorted(c for c in palette if normal(c) == pixel)) == 1:
            rows[name] = (matches[0], "-", "palette")
        elif full not in SPOILT and (made := between(full, pixel)):
            rows[name] = (made, "-", "between")
        else:
            rows[name] = (None, pixel, "odd")
    # A spoilt cell of an off-byte colour takes the colour of a block drawn the same.
    shared = {ground[name][0]: row[0] for name, row in rows.items() if row[2] == "between"}
    for name, row in rows.items():
        if row[2] == "odd" and row[1] in shared:
            rows[name] = (shared[row[1]], "-", "between")
        elif row[0] and row[0].startswith("="):
            rows[name] = rows[row[0][1:]][:2] + ("by hand",)
    odd = [name for name, row in rows.items() if row[2] == "odd"]
    assert not odd, f"blocks without a colour: {odd}"
    with open(OUT, "w", encoding="utf-8", newline="\n") as out:
        out.write("# block\tcolour (rrggbb, r,g,b, or - for none)\ttint\thow it was read\n")
        for name, (colour, tint, how) in rows.items():
            out.write(f"{name}\t{colour}\t{tint}\t{how}\n")
    print(len(rows), "blocks")


main()
