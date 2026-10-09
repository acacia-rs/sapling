"""BDS's vanilla behaviour packs -> fire-immune.txt: the entities whose own components hold
`minecraft:fire_immune`. See ../../README.md "Fire immune".

usage: py -3 fire-immune.py
"""
import importlib.util
import os

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("mob_loot", os.path.join(HERE, "mob-loot.py"))
packs = importlib.util.module_from_spec(spec)
spec.loader.exec_module(packs)

immune = set()
for document in packs.entities().values():
    entity = document.get("minecraft:entity")
    if entity and entity.get("components", {}).get("minecraft:fire_immune", False) is not False:
        immune.add(entity["description"]["identifier"])
open(os.path.join(HERE, "..", "fire-immune.txt"), "w", newline="\n").write("".join(name + "\n" for name in sorted(immune)))
print(len(immune), "fire immune")
