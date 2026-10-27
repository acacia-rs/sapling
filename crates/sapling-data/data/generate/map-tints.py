"""BDS's tinted map colours -> map-tints.tsv: each overworld biome's grass, foliage and water tint, as the
map-colours pack read them off a grass block, oak leaves and water set in that biome
(research/captures/bds-maptints.cells). A biome /locate did not find (the old hills and mutated
biomes today's worldgen never makes) takes the tints of a found biome of the same climate in BDS's
behaviour packs. See ../../README.md "Map colours".

usage: py -3 map-tints.py
"""
import importlib.util
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
CELLS = os.path.join(HERE, "..", "..", "..", "..", "research", "captures", "bds-maptints.cells")
OUT = os.path.join(HERE, "..", "map-tints.tsv")
spec = importlib.util.spec_from_file_location("mob_loot", os.path.join(HERE, "mob-loot.py"))
packs = importlib.util.module_from_spec(spec)
spec.loader.exec_module(packs)

SAMPLES = ("grass_block", "oak_leaves", "water")


def tint(component):
    """The biome colour a sample's own was multiplied by, each channel 0 to 255."""
    own, _, tinted = component.split("|")
    pairs = zip(own.split(",")[:3], tinted.split(",")[:3])
    return ",".join(f"{round(float(t) * 255 / float(o), 4):g}" for o, t in pairs)


def probed():
    found = {}
    for line in open(CELLS, encoding="utf-8"):
        row = re.match(r"MAPCOL tint (\S+) at -?\d+ -?\d+ -?\d+ (.*)", line)
        if row:
            parts = dict(re.findall(r"(\w+) comp=(\S+)", row.group(2)))
            found[row.group(1)] = [tint(parts[sample]) for sample in SAMPLES]
    return found


def climates():
    found = {}
    for pack in packs.packs():
        path = os.path.join(packs.PACKS, pack, "__brarchive", "biomes.brarchive")
        if os.path.exists(path):
            for name, raw in packs.brarchive(path):
                components = packs.jsonc(raw).get("minecraft:biome", {}).get("components", {})
                if "minecraft:overworld_generation_rules" in components or "minecraft:climate" in components:
                    climate = components.get("minecraft:climate", {})
                    found[name.removesuffix(".biome.json")] = (climate.get("temperature"), climate.get("downfall"), str(components.get("minecraft:map_tints")))
    return found


def main():
    tints, climate = probed(), climates()
    rows = dict(tints)
    for name in sorted(climate):
        if name in rows or name in ("hell", "the_end", "basalt_deltas", "crimson_forest", "soulsand_valley", "warped_forest"):
            continue
        twins = sorted(other for other in tints if climate.get(other) == climate[name])
        if twins:
            rows[name] = tints[twins[0]]
        else:
            print("no tints for", name)
    with open(OUT, "w", encoding="utf-8", newline="\n") as out:
        out.write("# biome\tgrass\tfoliage\twater (each r,g,b, 0 to 255)\n")
        for name in sorted(rows):
            out.write(f"{name}\t" + "\t".join(rows[name]) + "\n")
    print(len(tints), "probed,", len(rows), "biomes")


main()
