"""BDS's vanilla behaviour packs -> mob-events.tsv: each entity's `events` by the names its definition
gives them, what `/event entity` may fire. See ../../README.md "Mob events".

usage: python3 mob-events.py
"""
import importlib.util
import os

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("mob_loot", os.path.join(HERE, "mob-loot.py"))
packs = importlib.util.module_from_spec(spec)
spec.loader.exec_module(packs)


def main():
    rows = {}
    for document in packs.entities().values():
        entity = document.get("minecraft:entity")
        if entity:
            rows[entity["description"]["identifier"]] = list((entity.get("events") or {}).keys())
    with open(os.path.join(HERE, "..", "mob-events.tsv"), "w", newline="\n") as out:
        for name in sorted(rows):
            if rows[name]:
                out.write(f"{name}\t{' '.join(rows[name])}\n")


if __name__ == "__main__":
    main()
