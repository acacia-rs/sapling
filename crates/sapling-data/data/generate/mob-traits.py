"""BDS's vanilla behaviour packs -> mob-traits.tsv: what each entity's own components (not its
component groups) say of fire, falls and breath, the experience a grown one leaves, how fast one
that walks strolls and flees, and what was drilled of the kinds that hunt. See ../../README.md "Mob traits".

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


def pace(entity, behaviour):
    """The speed a grown one that walks goes at under `behaviour`, or None for a kind without it."""
    holders = [("", entity.get("components", {}))] + list(entity.get("component_groups", {}).items())
    find = lambda key: next((held[key] for group, held in holders if key in held and "baby" not in group), None)
    goal, walks, movement = find("minecraft:behavior." + behaviour), find("minecraft:navigation.walk"), find("minecraft:movement")
    if goal is None or walks is None or not isinstance(movement, dict) or not isinstance(movement.get("value"), (int, float)):
        return None
    return round(movement["value"] * goal.get("speed_multiplier", 1), 4)


# Drilled on BDS, not read from the packs (docs/research/mob-attacks.md, "Other kinds").
HUNTS = {
    "minecraft:cave_spider": "attack=2 speed=0.3 sees=16",
    "minecraft:creeper": "attack=0 speed=0.25 sees=16 explodes=3",
    "minecraft:husk": "attack=3 speed=0.23 sees=35 hits=hunger:600",
    "minecraft:silverfish": "attack=1 speed=0.25 sees=8",
    "minecraft:skeleton": "attack=0 speed=0.25 sees=16 shoots=61",
    "minecraft:spider": "attack=2 speed=0.3 sees=16",
    "minecraft:vindicator": "attack=13 speed=0.35 sees=12",
    "minecraft:wither_skeleton": "attack=9 speed=0.3125 sees=16 hits=wither:200",
    "minecraft:wolf": "attack=3 speed=0.3 sees=16 provoked",
    "minecraft:zombie": "attack=3 speed=0.23 sees=35",
    "minecraft:zombie_pigman": "attack=9 speed=0.23 sees=35 provoked",
    "minecraft:zombie_villager": "attack=3 speed=0.23 sees=35",
    "minecraft:zombie_villager_v2": "attack=3 speed=0.23 sees=35",
}

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
    for trait, behaviour in [("strolls", "random_stroll"), ("panics", "panic")]:
        if entity["description"]["identifier"] != "minecraft:player" and pace(entity, behaviour) is not None:
            held.append(f"{trait}={pace(entity, behaviour):g}")
    held.extend(filter(None, [HUNTS.get(entity["description"]["identifier"])]))
    if held:
        rows.append((entity["description"]["identifier"], " ".join(held)))
open(os.path.join(HERE, "..", "mob-traits.tsv"), "w", newline="\n").write("".join(f"{name}\t{traits}\n" for name, traits in sorted(rows)))
print(len(rows), "entities with traits")
