"""BDS's vanilla behaviour packs -> mob-loot.json: each entity's loot table and every table those
name, as a world on BDS 1.26.52's newest base game version resolves them. See ../../README.md "Mob loot".

usage: py -3 mob-loot.py [--survey]
"""
import json
import os
import re
import struct
import sys

PACKS = "C:/Users/hexay/acacia/acacia/.testserver/bds-blocks/behavior_packs/"
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "mob-loot.json")


def version(pack):
    """`vanilla` is the base; `vanilla_1.21.130` and the like override it in version order."""
    tail = pack.removeprefix("vanilla").lstrip("_")
    return tuple(int(p) for p in tail.split(".")) if tail else ()


def packs():
    return sorted((p for p in os.listdir(PACKS) if re.fullmatch(r"vanilla(_[\d.]+)?", p)), key=version)


def brarchive(path):
    """The files of a `.brarchive`: a header, 256-byte entries (name, offset, length), then the bodies."""
    data = open(path, "rb").read()
    (count,) = struct.unpack_from("<I", data, 8)
    base = 16 + count * 256
    for i in range(count):
        entry = 16 + i * 256
        name = data[entry + 1:entry + 1 + data[entry]].decode()
        offset, length = struct.unpack_from("<II", data, entry + 248)
        yield name, data[base + offset:base + offset + length]


def jsonc(text):
    """JSON that may hold // and /* */ comments and trailing commas."""
    if isinstance(text, bytes):
        text = text.decode("utf-8-sig")
    out, i = [], 0
    while i < len(text):
        if text[i] == '"':
            j = i + 1
            while text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            out.append(text[i:j + 1])
            i = j + 1
        elif text.startswith("//", i):
            i = text.find("\n", i) if "\n" in text[i:] else len(text)
        elif text.startswith("/*", i):
            i = text.find("*/", i) + 2
        else:
            out.append(text[i])
            i += 1
    return json.loads(re.sub(r",(\s*[}\]])", r"\1", "".join(out)))


def entities():
    """Entity documents by file name, a later pack's replacing an earlier one's."""
    found = {}
    for pack in packs():
        path = os.path.join(PACKS, pack, "__brarchive", "entities.brarchive")
        if os.path.exists(path):
            found.update((name, jsonc(raw)) for name, raw in brarchive(path))
    return found


def loot_tables():
    """Loot tables by their path in a pack (`loot_tables/entities/chicken.json`), the newest pack's:
    loose, or in `__brarchive/loot_tables/entities.brarchive` and the like (the archive's path, less
    its extension, is the folder its files are in)."""
    found = {}
    for pack in packs():
        root = os.path.join(PACKS, pack)
        for here, _, files in os.walk(os.path.join(root, "loot_tables")):
            for name in files:
                path = os.path.join(here, name)
                found[os.path.relpath(path, root).replace("\\", "/")] = open(path, "rb").read()
        archives = os.path.join(root, "__brarchive")
        for here, _, files in os.walk(archives):
            for name in files:
                folder = os.path.relpath(os.path.join(here, name.removesuffix(".brarchive")), archives).replace("\\", "/")
                if name.endswith(".brarchive") and folder.startswith("loot_tables"):
                    found.update((f"{folder}/{inner}", raw) for inner, raw in brarchive(os.path.join(here, name)))
    return found


def entity_loot():
    """identifier -> (table its base components name or None, {group: table})."""
    out = {}
    for document in entities().values():
        entity = document.get("minecraft:entity")
        if not entity:
            continue
        name = entity["description"]["identifier"]
        base = entity.get("components", {}).get("minecraft:loot", {}).get("table")
        groups = {g: v["minecraft:loot"]["table"] for g, v in entity.get("component_groups", {}).items() if "minecraft:loot" in v}
        out[name] = (base, groups)
    return out


def default_table(base, groups):
    """The table of an adult in its default state: its own, or that of the group naming it so (a
    sheep with its wool, a plain zombie, a salmon of normal size). Slimes and magma cubes, whose
    loot depends on their size, have none."""
    if base:
        return base
    for group, table in groups.items():
        if re.search(r"(^|[:_])adult$", group) or group in ("minecraft:loot_wooly", "minecraft:zombie_default", "scale_normal"):
            return table
    return None


# Old item names the tables still use -> today's (checked by `mob_loot.rs`'s tests against ItemRegistry.bin).
RENAMES = {
    "minecraft:muttonRaw": "minecraft:mutton", "minecraft:netherStar": "minecraft:nether_star", "minecraft:clownfish": "minecraft:tropical_fish",
    "minecraft:fish": "minecraft:cod", "minecraft:totem": "minecraft:totem_of_undying",
}
# (name, data) -> the item that data now names; a sheep's wool is its colour's, white by default.
BY_DATA = {("minecraft:dye", 0): "minecraft:ink_sac", ("minecraft:skull", 1): "minecraft:wither_skeleton_skull", ("minecraft:wool", "colour"): "minecraft:white_wool"}
# Kinds whose table is not what they leave when they die.
NOT_MOBS = {"minecraft:fishing_hook", "minecraft:player", "minecraft:npc", "minecraft:tripod_camera"}


def renamed(entry):
    """`entry` under today's item name, the data function that picked the item dropped."""
    name, functions = entry["name"], entry.get("functions", [])
    for function in functions:
        data = "colour" if function["function"] == "minecraft:set_data_from_color_index" else function.get("data") if function["function"] == "set_data" else None
        if (name, data) in BY_DATA:
            return {**entry, "name": BY_DATA[(name, data)], "functions": [f for f in functions if f is not function]}
    if name.startswith("minecraft:record_"):
        name = "minecraft:music_disc_" + name.removeprefix("minecraft:record_")
    return {**entry, "name": RENAMES.get(name, name)}


def named_tables(table, tables, into):
    """Adds `table` and the tables its entries name to `into`, its items renamed."""
    if table in into or table not in tables:
        return
    into[table] = jsonc(tables[table])
    for pool in into[table].get("pools", []):
        pool["entries"] = [renamed(entry) if entry.get("type") == "item" else entry for entry in pool.get("entries", [])]
        for entry in pool["entries"]:
            if entry.get("type") == "loot_table":
                named_tables(entry["name"], tables, into)


def main():
    loot = entity_loot()
    if "--survey" in sys.argv:
        for name, (base, groups) in sorted(loot.items()):
            if base or groups:
                print(name, base, groups)
        return
    tables = loot_tables()
    found, used = {}, {}
    for name, (base, groups) in sorted(loot.items()):
        if name in NOT_MOBS:
            continue
        table = default_table(base, groups)
        if table in tables:
            found[name] = table
            named_tables(table, tables, used)
        elif table:
            print(f"{name}: no file for {table}")
    with open(OUT, "w", encoding="utf-8", newline="\n") as out:
        json.dump({"entities": found, "tables": dict(sorted(used.items()))}, out, separators=(",", ":"), sort_keys=True)
    print(f"{len(found)} entities, {len(used)} tables -> {OUT}")


if __name__ == "__main__":
    main()
