"""BDS's vanilla behaviour packs -> mob-families.tsv: each entity's `minecraft:type_family`, what a
selector's `family=` tests. See ../../README.md "Mob families".

usage: py -3 mob-families.py
"""
import importlib.util
import os

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("mob_loot", os.path.join(HERE, "mob-loot.py"))
packs = importlib.util.module_from_spec(spec)
spec.loader.exec_module(packs)


def families(entity):
    """Its own components' families; for a kind that only has them in groups (a villager's
    profession, a baby's), every group's together."""
    family = lambda held: held.get("minecraft:type_family", {}).get("family", [])
    own = family(entity.get("components", {}))
    if own:
        return own
    grouped = []
    for held in entity.get("component_groups", {}).values():
        grouped += [name for name in family(held) if name not in grouped]
    return grouped


def main():
    rows = {}
    for document in packs.entities().values():
        entity = document.get("minecraft:entity")
        if entity:
            rows[entity["description"]["identifier"]] = families(entity)
    with open(os.path.join(HERE, "..", "mob-families.tsv"), "w", newline="\n") as out:
        for name in sorted(rows):
            if rows[name]:
                out.write(f"{name}\t{' '.join(rows[name])}\n")


if __name__ == "__main__":
    main()
