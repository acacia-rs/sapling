"""BDS's vanilla behaviour packs -> husbandry.tsv: what each entity's breedable, ageable, tameable,
tamemount, healable, spawn_entity, follow_owner, leashable and tempt components say, and the
component groups its born, grow-up, tame, shear and regrow events swap. See ../../README.md "Husbandry".

usage: py -3 husbandry.py
"""
import importlib.util
import os

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("mob_loot", os.path.join(HERE, "mob-loot.py"))
packs = importlib.util.module_from_spec(spec)
spec.loader.exec_module(packs)

# Old item names the packs still use -> today's (checked by `husbandry.rs`'s tests against ItemRegistry.bin).
RENAMES = {
    "muttonRaw": "mutton", "muttonCooked": "cooked_mutton", "appleEnchanted": "enchanted_golden_apple", "appleenchanted": "enchanted_golden_apple",
    "fish": "cod", "clownfish": "tropical_fish", "cooked_fish": "cooked_cod", "horsearmorleather": "leather_horse_armor",
    "horsearmoriron": "iron_horse_armor", "horsearmorgold": "golden_horse_armor", "horsearmordiamond": "diamond_horse_armor",
    "carrotOnAStick": "carrot_on_a_stick",
}
# `minecraft:ageable`'s growth for an item that names none (Bedrock's add-on documentation).
GROWTH = 0.1
EVENTS = {"on_born": "minecraft:entity_born", "on_grow": "minecraft:ageable_grow_up", "on_tame": "minecraft:on_tame", "on_sheared": "minecraft:on_sheared", "on_regrow": "minecraft:on_eat_block"}


def item(entry):
    """An item as the packs name it (bare, namespaced or `{"item": ..}`), namespaced under today's name; data dropped."""
    name = entry["item"] if isinstance(entry, dict) else entry
    name = name.removeprefix("minecraft:").split(":")[0]
    return "minecraft:" + RENAMES.get(name, name)


def listed(value):
    return value if isinstance(value, list) else [value]


def found(entity, key):
    """The component `key` of the entity's own components, else of its first group holding one."""
    holders = [entity.get("components", {})] + list(entity.get("component_groups", {}).values())
    return next((held[key] for held in holders if key in held), None)


def swaps(entity, event, depth=0):
    """The groups an event removes and adds, followed through its triggers and unconditional
    sequence steps; randomized and filtered steps are left out."""
    body = entity.get("events", {}).get(event)
    if body is None or depth > 4:
        return [], []
    removed, added = [], []

    def take(step):
        if "filters" in step:
            return
        removed.extend(step.get("remove", {}).get("component_groups", []))
        added.extend(step.get("add", {}).get("component_groups", []))
        if "trigger" in step:
            trigger = step["trigger"]
            more = swaps(entity, trigger if isinstance(trigger, str) else trigger.get("event"), depth + 1)
            removed.extend(more[0])
            added.extend(more[1])
        for inner in step.get("sequence", []):
            take(inner)

    take(body)
    return removed, added


def row(entity):
    fields = []
    breedable = found(entity, "minecraft:breedable")
    if breedable:
        fields.append("breed=" + ",".join(item(i) for i in listed(breedable.get("breed_items", []))))
        fields.append("mates=" + ",".join(sorted(breedable.get("breeds_with", {}))))
        fields += ["breed_tame"] * bool(breedable.get("require_tame")) + ["breed_healthy"] * bool(breedable.get("require_full_health"))
    ageable = found(entity, "minecraft:ageable")
    if ageable:
        feed = [f"{item(i)}:{i.get('growth', GROWTH) if isinstance(i, dict) else GROWTH}" for i in listed(ageable.get("feed_items", []))]
        fields.append(f"grows={int(ageable.get('duration', 1200) * 20)}")
        if feed:
            fields.append("grow=" + ",".join(feed))
    tameable = found(entity, "minecraft:tameable")
    if tameable:
        fields.append("tame=" + ",".join(item(i) for i in listed(tameable.get("tame_items", []))))
        fields.append(f"tame_chance={tameable.get('probability', 1.0)}")
    mount = found(entity, "minecraft:tamemount")
    if mount:
        fields.append(f"temper={mount.get('min_temper', 0)}-{mount.get('max_temper', 100)}")
        fields.append("temper_feed=" + ",".join(f"{item(i)}:{i.get('temper_mod', 0)}" for i in mount.get("feed_items", [])))
    healable = found(entity, "minecraft:healable")
    if healable and healable.get("items"):
        # A parrot's cookie heals nothing: it poisons.
        heals = [f"{item(i)}:{i.get('heal_amount', 1)}" for i in healable["items"] if i.get("heal_amount", 1) > 0]
        fields += ["heal=" + ",".join(heals)] * bool(heals)
    lays = found(entity, "minecraft:spawn_entity")
    lays = next((e for e in listed(lays.get("entities", [])) if e.get("spawn_item") and not e.get("single_use") and e.get("min_wait_time", 0) > 0), None) if lays else None
    if lays:
        fields.append(f"lays={item(lays['spawn_item'])}:{lays['min_wait_time'] * 20}-{lays['max_wait_time'] * 20}")
    follows = found(entity, "minecraft:behavior.follow_owner")
    if follows:
        fields.append(f"follows={follows.get('start_distance', 10)}-{follows.get('stop_distance', 2)}")
    fields += ["sits"] * (found(entity, "minecraft:sittable") is not None)
    fields += ["defends"] * (found(entity, "minecraft:behavior.owner_hurt_by_target") is not None)
    fields += ["leashable"] * (found(entity, "minecraft:leashable") is not None)
    tempt = found(entity, "minecraft:behavior.tempt")
    if tempt and tempt.get("items"):
        fields.append("tempt=" + ",".join(item(i) for i in listed(tempt["items"])))
        fields.append(f"tempt_speed={tempt.get('speed_multiplier', 1.0)}")
    movement = found(entity, "minecraft:movement")
    movement = movement.get("value") if isinstance(movement, dict) else None
    # A horse's is a range it rolls in: the middle of it.
    movement = (movement["range_min"] + movement["range_max"]) / 2 if isinstance(movement, dict) else movement
    if isinstance(movement, (int, float)) and fields:
        fields.append(f"walks={round(movement, 4)}")
    if not fields:
        return None
    groups = entity.get("component_groups", {})
    tamed = [groups.get(g, {}) for g in swaps(entity, EVENTS["on_tame"])[1]]
    # A tamed wolf's health comes from a group of its own that its upgrade event gives.
    tamed += [held for name, held in groups.items() if name.endswith("increased_max_health")]
    health = next((held["minecraft:health"].get("max") for held in reversed(tamed) if "minecraft:health" in held), None)
    attack = next((held["minecraft:attack"].get("damage") for held in tamed if "minecraft:attack" in held), None)
    fields += [f"tame_health={health}"] * (health is not None) + [f"tame_attack={attack}"] * isinstance(attack, (int, float))
    for name, event in EVENTS.items():
        removed, added = swaps(entity, event)
        # A born event that only branches on filters (a wolf's): the baby has the groups growing up takes away.
        if name == "on_born" and not added and ageable:
            added = swaps(entity, EVENTS["on_grow"])[0]
        if removed or added:
            fields.append(f"{name}=" + ",".join(["-" + g for g in removed] + ["+" + g for g in added]))
    return " ".join(fields)


def main():
    rows = []
    for document in packs.entities().values():
        entity = document.get("minecraft:entity")
        if entity and (held := row(entity)):
            rows.append((entity["description"]["identifier"], held))
    out = os.path.join(HERE, "..", "husbandry.tsv")
    open(out, "w", newline="\n").write("".join(f"{name}\t{fields}\n" for name, fields in sorted(rows)))
    print(len(rows), "entities ->", out)


if __name__ == "__main__":
    main()
