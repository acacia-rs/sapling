"""BDS's vanilla behaviour packs -> entity-properties.tsv: each entity's `description.properties` that
clients are told of (`client_sync`), by name within a kind as BDS numbers them, the kinds in the order
BDS 1.26.52 sent their SyncEntityProperty at join (acacia-testserver/fixtures/bds-1.26.52.script).
See ../../README.md "Entity properties".

usage: py -3 entity-properties.py
"""
import importlib.util
import os

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("mob_loot", os.path.join(HERE, "mob-loot.py"))
packs = importlib.util.module_from_spec(spec)
spec.loader.exec_module(packs)

JOIN_ORDER = [
    "zombie_nautilus", "wolf", "sulfur_cube", "chicken", "creaking", "pig", "cat", "egg", "armadillo", "happy_ghast", "copper_golem", "bee", "cow",
]


def text(value):
    return str(value).lower() if isinstance(value, bool) else str(value)


def row(kind, name, spec):
    kind_of = spec["type"]
    if kind_of == "enum":
        values = ",".join(spec["values"])
    elif kind_of in ("int", "float"):
        values = "..".join(text(bound) for bound in spec["range"])
    else:
        values = "-"
    return f"{kind}\t{name}\t{kind_of}\t{values}\t{text(spec.get('default', ''))}\n"


def main():
    kinds = {}
    for document in packs.entities().values():
        entity = document.get("minecraft:entity", {})
        description = entity.get("description", {})
        synced = {name: spec for name, spec in description.get("properties", {}).items() if spec.get("client_sync")}
        if synced:
            kinds[description["identifier"]] = synced
    order = [f"minecraft:{kind}" for kind in JOIN_ORDER]
    order += sorted(kind for kind in kinds if kind not in order)
    with open(os.path.join(HERE, "..", "entity-properties.tsv"), "w", newline="\n") as out:
        for kind in order:
            for name in sorted(kinds.get(kind, {})):
                out.write(row(kind, name, kinds[kind][name]))


if __name__ == "__main__":
    main()
