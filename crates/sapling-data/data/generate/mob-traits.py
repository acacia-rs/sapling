"""BDS's vanilla behaviour packs -> mob-traits.tsv: what each entity's own components (not its
component groups) say of fire, falls and breath, and the experience a grown one leaves. See ../../README.md "Mob traits".

usage: py -3 mob-traits.py
"""
import importlib.util
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("mob_loot", os.path.join(HERE, "mob-loot.py"))
packs = importlib.util.module_from_spec(spec)
spec.loader.exec_module(packs)


def unhurt_by(components, cause):
    triggers = components.get("minecraft:damage_sensor", {}).get("triggers", [])
    triggers = triggers if isinstance(triggers, list) else [triggers]
    return any(t.get("cause") in (cause, "all") and t.get("deals_damage") in (False, "no") and "on_damage" not in t for t in triggers)


def experience(entity):
    """`least-most` of what a grown one without equipment leaves a player who killed it."""
    holders = [("", entity.get("components", {}))] + list(entity.get("component_groups", {}).items())
    grown = [held["minecraft:experience_reward"].get("on_death") for group, held in holders if "minecraft:experience_reward" in held and "baby" not in group]
    if not grown:
        return None
    reward = str(grown[0])
    reward = reward.split("?", 1)[1] if "last_hit_by_player" in reward else reward
    reward = reward.replace("(query.is_baby ? 12 : 5)", "5")
    rolled = re.match(r"\s*Math\.Random\((\d+),\s*(\d+)\)", reward)
    fixed = re.match(r"\s*(\d+)", reward)
    return f"{rolled[1]}-{rolled[2]}" if rolled else f"{fixed[1]}-{fixed[1]}" if fixed else None


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
    if entity["description"]["identifier"] != "minecraft:player" and experience(entity):
        held.append("xp=" + experience(entity))
    if held:
        rows.append((entity["description"]["identifier"], " ".join(held)))
open(os.path.join(HERE, "..", "mob-traits.tsv"), "w", newline="\n").write("".join(f"{name}\t{traits}\n" for name, traits in sorted(rows)))
print(len(rows), "entities with traits")
