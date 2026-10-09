"""BDS's vanilla behaviour packs -> mob-traits.tsv: what each entity's own components (not its
component groups) say of fire, falls and breath. See ../../README.md "Mob traits".

usage: py -3 mob-traits.py
"""
import importlib.util
import os

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("mob_loot", os.path.join(HERE, "mob-loot.py"))
packs = importlib.util.module_from_spec(spec)
spec.loader.exec_module(packs)


def unhurt_by(components, cause):
    triggers = components.get("minecraft:damage_sensor", {}).get("triggers", [])
    triggers = triggers if isinstance(triggers, list) else [triggers]
    return any(t.get("cause") in (cause, "all") and t.get("deals_damage") in (False, "no") and "on_damage" not in t for t in triggers)


rows = []
for document in packs.entities().values():
    entity = document.get("minecraft:entity")
    if not entity:
        continue
    components = entity.get("components", {})
    breath = components.get("minecraft:breathable", {})
    traits = [
        ("fire_immune", components.get("minecraft:fire_immune", False) is not False),
        ("no_fall", unhurt_by(components, "fall")),
        ("breathes_water", breath.get("breathes_water", False)),
        ("no_air", breath.get("breathes_air", True) is False),
    ]
    held = [name for name, holds in traits if holds]
    if held:
        rows.append((entity["description"]["identifier"], " ".join(held)))
open(os.path.join(HERE, "..", "mob-traits.tsv"), "w", newline="\n").write("".join(f"{name}\t{traits}\n" for name, traits in sorted(rows)))
print(len(rows), "entities with traits")
