"""Item tags named by Bedrock recipes -> item-tags.tsv. See loot-and-tags.md.

usage: py -3 item-tags.py
"""
import json
import os
import sys
import zipfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from bedrock_names import load_bedrock_names, to_bedrock  # noqa: E402

JAR = "C:/Users/Hexay/acacia/research/java-truth/client-26.3.jar"
DUMP = os.path.join(HERE, "pmmp-item_tags-1.26.30.json")
TAGS = "data/minecraft/tags/item/"
# Bedrock tag -> Java item tag file (None: Java has no such tag).
JAVA_TAG = {
    "minecraft:coals": "coals",
    "minecraft:decorated_pot_sherds": "decorated_pot_sherds",
    "minecraft:egg": "eggs",
    "minecraft:is_pickaxe": "pickaxes",
    "minecraft:logs": "logs",
    "minecraft:logs_that_burn": "logs_that_burn",
    "minecraft:metal_nuggets": "metal_nuggets",
    "minecraft:mushrooms_for_stew": "mushrooms",
    "minecraft:planks": "planks",
    "minecraft:soul_fire_base_blocks": "soul_fire_base_blocks",
    "minecraft:stone_crafting_materials": "stone_crafting_materials",
    "minecraft:stone_tool_materials": "stone_tool_materials",
    "minecraft:trim_materials": "trim_materials",
    "minecraft:trim_templates": None,
    "minecraft:trimmable_armors": "trimmable_armor",
    "minecraft:wooden_slabs": "wooden_slabs",
    "minecraft:wool": "wool",
}
# Named in the recipe list as a crafting-station tag (SmithingTransform/SmithingTrim `tag` field), not an item tag.
NOT_ITEM_TAGS = {"smithing_table"}


def expand(z, tag, seen=()):
    """-> [(java item, tag file that names it directly)], nested tags expanded."""
    out = []
    for v in json.loads(z.read(TAGS + tag + ".json"))["values"]:
        v = v["id"] if isinstance(v, dict) else v
        if v.startswith("#"):
            sub = v[1:].replace("minecraft:", "")
            assert sub not in seen
            out += expand(z, sub, seen + (tag,))
        else:
            out.append((v.replace("minecraft:", ""), tag))
    return out


def main():
    names = load_bedrock_names()
    z = zipfile.ZipFile(JAR)
    java_files = {n[len(TAGS):-5] for n in z.namelist() if n.startswith(TAGS) and n.endswith(".json")}
    with open(DUMP, encoding="utf-8") as f:
        dump = json.load(f)
    dumped_items = {i for members in dump.values() for i in members}
    with open(os.path.join(HERE, "recipe-tags-used.txt"), encoding="utf-8") as f:
        wanted = [line.strip() for line in f if line.strip()]
    rows = []
    for tag in wanted:
        if tag in NOT_ITEM_TAGS:
            print(f"{tag}: not an item tag, no rows")
            continue
        jt = JAVA_TAG[tag]
        assert jt is None or jt in java_files, jt
        assert jt is not None or tag.replace("minecraft:", "") not in java_files
        java, unmapped = {}, []
        for item, src in (expand(z, jt) if jt else []):
            b = to_bedrock(item, names)
            if b:
                java.setdefault("minecraft:" + b, "java-tag:" + src + ".json")
            else:
                unmapped.append(item)
        bedrock = set(dump.get(tag, []))
        members = set(java) | bedrock
        dropped = sorted(m for m in members if m not in names)
        mine = []
        for m in sorted(members - set(dropped)):
            if tag not in dump:
                in_dump = "tag_not_in_dump"
            elif m in bedrock:
                in_dump = "yes"
            else:
                in_dump = "no" if m in dumped_items else "item_not_in_dump"
            mine.append((tag, m, java.get(m, "bedrock-dump"), in_dump))
        rows += mine
        print(f"{tag}: rows={len(mine)} java={jt} " + " ".join(f"{k}={sum(1 for r in mine if r[3] == k)}" for k in sorted({r[3] for r in mine})))
        for label, v in (("not_confirmed_by_dump", [r[1] for r in mine if r[3] != "yes"]),
                         ("dump_only", [r[1] for r in mine if r[2] == "bedrock-dump"]),
                         ("java_unmapped", unmapped), ("not_a_bedrock_item", dropped)):
            if v:
                print(f"   {label}({len(v)}): {' '.join(x.replace('minecraft:', '') for x in v)}"[:700])
    with open(os.path.join(HERE, "item-tags.tsv"), "w", encoding="utf-8", newline="\n") as f:
        f.write("tag\titem\tsource\tin_bedrock_dump\n")
        for r in rows:
            f.write("\t".join(r) + "\n")
    print("rows", len(rows))


if __name__ == "__main__":
    main()
