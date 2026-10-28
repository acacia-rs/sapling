"""BDS's vanilla behaviour packs -> spawn-rules.tsv and despawns.tsv: each natural spawn condition of
`spawn_rules/*.json`, and the distances of each entity's `minecraft:despawn`. See ../../README.md "Spawn rules".

usage: py -3 spawn-rules.py
"""
import importlib.util
import os

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("mob_loot", os.path.join(HERE, "mob-loot.py"))
packs = importlib.util.module_from_spec(spec)
spec.loader.exec_module(packs)

# Conditions holding these need what sapling has not built (patrols, villages); they are left out.
UNBUILT = ("minecraft:mob_event_filter", "minecraft:delay_filter", "minecraft:world_age_filter", "minecraft:player_in_village_filter")
# Kinds whose rules BDS's own code gates (ocean monuments): left out too. Slimes' and phantoms' gates
# are sapling-shard's spawning/gates.rs and phantoms.rs.
GATED = ("minecraft:guardian",)
PLACES = {"minecraft:spawns_on_surface": "surface", "minecraft:spawns_underground": "underground", "minecraft:spawns_underwater": "underwater", "minecraft:spawns_lava": "lava"}
# Old block names the rules still use -> today's.
RENAMES = {"minecraft:grass": "minecraft:grass_block"}
# The despawn component's defaults (Bedrock's add-on documentation of `minecraft:despawn`).
NEAR, FAR = 32, 128


def spawn_rules():
    """Spawn rule documents by file name, a later pack's replacing an earlier one's."""
    found = {}
    for pack in packs.packs():
        path = os.path.join(packs.PACKS, pack, "__brarchive", "spawn_rules.brarchive")
        if os.path.exists(path):
            found.update((name, packs.jsonc(raw)) for name, raw in packs.brarchive(path))
    return found


def biome(test):
    """A biome filter as a prefix expression: `tag`, `!tag`, `&(..)`, `|(..)`; `snow` for `is_snow_covered`."""
    if isinstance(test, list):
        return f"&({' '.join(map(biome, test))})"
    for key, op in (("all_of", "&"), ("any_of", "|"), ("none_of", "!|")):
        if key in test:
            return f"{op}({' '.join(map(biome, test[key]))})"
    name = {"has_biome_tag": test.get("value"), "is_snow_covered": "snow"}[test["test"]]
    value = test.get("value", True) if test["test"] == "is_snow_covered" else True
    negated = (test.get("operator", "==") in ("!=", "not")) != (value is False)
    return ("!" if negated else "") + name


def blocks(listed):
    """Block names, given bare or as `{"name": ..}`."""
    listed = listed if isinstance(listed, list) else [listed]
    names = [block["name"] if isinstance(block, dict) else block for block in listed]
    return ",".join(RENAMES.get(name, name) for name in names)


def condition(kind, category, held):
    places = [place for key, place in PLACES.items() if key in held]
    herd = held.get("minecraft:herd", {})
    herd = herd[0] if isinstance(herd, list) else herd
    fields = []
    if "minecraft:brightness_filter" in held:
        light = held["minecraft:brightness_filter"]
        fields.append(f"light={light.get('min', 0)}~{light.get('max', 15)}" + ("w" if light.get("adjust_for_weather") else ""))
    for key, name in (("minecraft:difficulty_filter", "difficulty"), ("minecraft:height_filter", "height"), ("minecraft:distance_filter", "distance")):
        if key in held:
            fields.append(f"{name}={held[key]['min']}~{held[key]['max']}")
    if "minecraft:density_limit" in held:
        limit = held["minecraft:density_limit"]
        fields.append(f"density={limit.get('surface', '-')}:{limit.get('underground', '-')}")
    if "minecraft:spawns_on_block_filter" in held:
        fields.append("on=" + blocks(held["minecraft:spawns_on_block_filter"]))
    if "minecraft:spawns_on_block_prevented_filter" in held:
        fields.append("not_on=" + blocks(held["minecraft:spawns_on_block_prevented_filter"]))
    if "minecraft:permute_type" in held:
        fields.append("permute=" + ",".join(f"{p['weight']}:{p.get('entity_type', kind)}" for p in held["minecraft:permute_type"]))
    if "minecraft:disallow_spawns_in_bubble" in held:
        fields.append("no_bubble")
    weight = held.get("minecraft:weight", {}).get("default", 1)
    size = f"{herd.get('min_size', 1)}-{herd.get('max_size', 1)}"
    return f"{kind}\t{category}\t{','.join(places)}\t{weight}\t{size}\t{' '.join(fields)}\t{biome(held.get('minecraft:biome_filter', []))}\n"


def despawns():
    """identifier -> `near-far` of its own `minecraft:despawn` by distance (not one of a component group's)."""
    out = {}
    for document in packs.entities().values():
        entity = document.get("minecraft:entity")
        despawn = entity and entity.get("components", {}).get("minecraft:despawn")
        if despawn and "despawn_from_distance" in despawn:
            far = despawn["despawn_from_distance"]
            out[entity["description"]["identifier"]] = f"{far.get('min_distance', NEAR)}-{far.get('max_distance', FAR)}"
    return out


def main():
    rows, left = [], []
    for name, document in sorted(spawn_rules().items()):
        rules = document["minecraft:spawn_rules"]
        kind, category = rules["description"]["identifier"], rules["description"]["population_control"]
        for held in rules.get("conditions", []):
            if kind in GATED or any(key in held for key in UNBUILT):
                left.append(name)
                continue
            rows.append(condition(kind, category, held))
    with open(os.path.join(HERE, "..", "spawn-rules.tsv"), "w", newline="\n") as out:
        out.writelines(rows)
    with open(os.path.join(HERE, "..", "despawns.tsv"), "w", newline="\n") as out:
        out.writelines(f"{kind}\t{far}\n" for kind, far in sorted(despawns().items()))
    print(len(rows), "conditions; left out:", " ".join(sorted(set(left))))


if __name__ == "__main__":
    main()
