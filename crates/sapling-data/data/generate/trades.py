"""BDS's vanilla behaviour packs -> trades.json: the trade tables of `trading/economy_trades/*.json` that
villagers and the wandering trader name, the newest pack's file winning. See ../../README.md "Trades".

Some files ship compiled (`\\x7fMCB`, a binary form of the same table); `mcb` reads those. Every item is
written as a list of alternatives (a `choice`, or one), its name as today's item with its data value,
and its biome `filters` as the villager types (`MarkVariant`) they let through.

usage: py -3 trades.py [--survey]
"""
import importlib.util
import json
import os
import struct
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("mob_loot", os.path.join(HERE, "mob-loot.py"))
packs = importlib.util.module_from_spec(spec)
spec.loader.exec_module(packs)

OUT = os.path.join(HERE, "..", "trades.json")
FOLDER = "trading/economy_trades"
# (name, data) the tables still use -> today's item; other data values stay the item's `Damage`.
BY_DATA = {
    ("minecraft:dye", 0): "minecraft:ink_sac", ("minecraft:bucket", 2): "minecraft:cod_bucket", ("minecraft:bucket", 4): "minecraft:tropical_fish_bucket",
    ("minecraft:bucket", 5): "minecraft:pufferfish_bucket", ("minecraft:bucket", 10): "minecraft:lava_bucket", ("minecraft:tallgrass", 2): "minecraft:fern",
}
# Old names -> today's (checked by `trades.rs`'s tests against ItemRegistry.bin).
RENAMES = {
    "minecraft:reeds": "minecraft:sugar_cane", "minecraft:muttonraw": "minecraft:mutton", "minecraft:speckled_melon": "minecraft:glistering_melon_slice",
    "minecraft:turtle_shell_piece": "minecraft:turtle_scute", "minecraft:map": "minecraft:filled_map", "minecraft:horsearmorleather": "minecraft:leather_horse_armor",
    "minecraft:fish": "minecraft:cod", "minecraft:cooked_fish": "minecraft:cooked_cod", "minecraft:clownfish": "minecraft:tropical_fish",
}
# Items whose data value is no part of today's item: the tables name a log's old type this way.
NO_DATA = ("_log", "minecraft:coal")
MARKS = range(7)
# `set_potion`'s names -> the potion's data value (sapling-data `potion_effects`); a tipped arrow's is one more.
POTIONS = {
    "nightvision": 5, "invisibility": 7, "leaping": 9, "fire_resistance": 12, "swiftness": 14, "slowness": 17, "water_breathing": 19, "healing": 21, "harming": 23,
    "poison": 25, "regeneration": 28, "strength": 31, "weakness": 34, "wither": 36, "turtle_master": 37,
}


class Reader:
    def __init__(self, data):
        self.data, self.at = data, 0

    def take(self, fmt):
        (value,) = struct.unpack_from(fmt, self.data, self.at)
        self.at += struct.calcsize(fmt)
        return value

    def text(self):
        size = self.take("<B")
        self.at += size
        return self.data[self.at - size:self.at].decode()


def mcb(data):
    """A compiled table: tiers of groups of trades, each item a list of alternatives. Field order,
    found by reading the compiled files of tables also shipped as JSON (vanilla_1.21.40's wandering
    trader against vanilla_1.21.20's): per alternative min and max count, name, a byte, two ints and a
    byte always 0 2 0 0, the price multiplier; per trade trader_exp, max_uses, reward_exp, weight;
    per group num_to_select first; per tier total_exp_required last."""
    r = Reader(data)
    assert data[:4] == b"\x7fMCB"
    r.at = 8
    r.take("<i")
    assert r.text() == "tiers"

    def alternative():
        low, high, name = r.take("<i"), r.take("<i"), r.text()
        assert (r.take("<B"), r.take("<i"), r.take("<i"), r.take("<B")) == (0, 2, 0, 0), name
        return {"item": name, "quantity": {"min": low, "max": high}, "price_multiplier": r.take("<f")}

    def items():
        listed = []
        for _ in range(r.take("<B")):
            choice = [alternative() for _ in range(r.take("<B"))]
            listed.append(choice[0] if len(choice) == 1 else {"choice": choice})
        return listed

    def trade():
        wants, gives = items(), items()
        return {"wants": wants, "gives": gives, "trader_exp": r.take("<i"), "max_uses": r.take("<i"), "reward_exp": r.take("<B") != 0, "weight": r.take("<i")}

    tiers = []
    for _ in range(r.take("<B")):
        groups = []
        for _ in range(r.take("<B")):
            select = r.take("<i")
            groups.append({"num_to_select": select, "trades": [trade() for _ in range(r.take("<B"))]})
        tiers.append({"groups": groups, "total_exp_required": r.take("<i")})
    assert r.at == len(data)
    return {"tiers": tiers}


def tables():
    """Trade tables by file name, a later pack's replacing an earlier one's."""
    found = {}
    for pack in packs.packs():
        path = os.path.join(packs.PACKS, pack, "__brarchive", *FOLDER.split("/")) + ".brarchive"
        if os.path.exists(path):
            found.update((name, (pack, raw)) for name, raw in packs.brarchive(path))
    return {name: mcb(raw) if raw[:4] == b"\x7fMCB" else packs.jsonc(raw) for name, (_, raw) in found.items()}


def passes(test, mark):
    """Whether a villager of type `mark`, in the overworld, passes an item's `filters`."""
    if "all_of" in test:
        return all(passes(t, mark) for t in test["all_of"])
    if "any_of" in test:
        return any(passes(t, mark) for t in test["any_of"])
    held = {"is_mark_variant": lambda: mark == test["value"], "in_overworld": lambda: test.get("value", True) is True}[test["test"]]()
    return held != (test.get("operator", "=") in ("!=", "not"))


def item(entry):
    """One alternative: today's name and data value, the count range, and what else it carries."""
    parts = entry["item"].split(":")
    name, data = ":".join(parts[:2]), int(parts[2]) if len(parts) > 2 else None
    if (name, data) in BY_DATA:
        name, data = BY_DATA[(name, data)], None
    name = RENAMES.get(name, name)
    if data is not None and name.endswith(NO_DATA):
        data = None
    functions = [f for f in entry.get("functions", []) if f["function"] != "set_potion"]
    for potion in (f for f in entry.get("functions", []) if f["function"] == "set_potion"):
        data = POTIONS[potion["id"]] + (name == "minecraft:arrow")
    quantity = entry.get("quantity", 1)
    low, high = (quantity["min"], quantity["max"]) if isinstance(quantity, dict) else (quantity, quantity)
    out = {"name": name, "count": [low, high]}
    if data is not None:
        out["data"] = data
    if entry.get("price_multiplier"):
        out["price_multiplier"] = round(entry["price_multiplier"], 4)
    if functions:
        out["functions"] = functions
    if "filters" in entry:
        out["marks"] = [mark for mark in MARKS if passes(entry["filters"], mark)]
    return out


def trade(entry):
    alternatives = lambda listed: [[item(c) for c in e["choice"]] if "choice" in e else [item(e)] for e in listed]
    return {
        "wants": alternatives(entry["wants"]), "gives": alternatives(entry["gives"]), "trader_exp": entry.get("trader_exp", 1), "max_uses": entry.get("max_uses", 1),
        "reward_exp": entry.get("reward_exp", True), "weight": entry.get("weight", 1),
    }


def tier(entry):
    """A tier's groups; an old-style tier's bare `trades` are one group offering all of them."""
    groups = entry.get("groups") or [{"trades": entry.get("trades", [])}]
    return {"exp": entry.get("total_exp_required", 0), "groups": [{"select": g.get("num_to_select", -1), "trades": [trade(t) for t in g["trades"]]} for g in groups]}


def main():
    found = tables()
    if "--survey" in sys.argv:
        for name, table in sorted(found.items()):
            print(name, [(t.get("total_exp_required"), [len(g["trades"]) for g in t.get("groups", [])]) for t in table["tiers"]])
        return
    out = {name.removesuffix("_trades.json"): {"tiers": [tier(t) for t in table["tiers"]]} for name, table in sorted(found.items())}
    with open(OUT, "w", encoding="utf-8", newline="\n") as file:
        json.dump(out, file, separators=(",", ":"), sort_keys=True)
    print(f"{len(out)} tables -> {OUT}")


if __name__ == "__main__":
    main()
