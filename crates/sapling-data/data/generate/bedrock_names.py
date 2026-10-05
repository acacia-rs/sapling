"""Java item id -> Bedrock item id, checked against bedrock-item-names.txt. Shared by block-loot.py and item-tags.py."""
import os

HERE = os.path.dirname(os.path.abspath(__file__))

# Hand-written renames (no namespace). Every target is asserted to exist in bedrock-item-names.txt.
JAVA_TO_BEDROCK = {
    "cobweb": "web",
    "dead_bush": "deadbush",
    "flowering_azalea_leaves": "azalea_leaves_flowered",
    "melon": "melon_block",
    "nether_quartz_ore": "quartz_ore",
    "oak_door": "wooden_door",
    "small_dripleaf": "small_dripleaf_block",
    "snow_block": "snow",
    "stone_slab": "normal_stone_slab",
}
# Java name also exists on Bedrock but means a different item there; these win over the identity match.
COLLISIONS = {
    "snow": "snow_layer",
    "stone_stairs": "normal_stone_stairs",
    "stonecutter": "stonecutter_block",
    "nether_brick": "netherbrick",
}
# One Bedrock item with the colour in the aux value.
for _c in ("white orange magenta light_blue yellow lime pink gray light_gray cyan purple blue brown green red black").split():
    JAVA_TO_BEDROCK[_c + "_bed"] = "bed"
    JAVA_TO_BEDROCK[_c + "_banner"] = "banner"
JAVA_TO_BEDROCK.update(COLLISIONS)


def load_bedrock_names():
    with open(os.path.join(HERE, "bedrock-item-names.txt"), encoding="utf-8") as f:
        names = {line.strip() for line in f if line.strip()}
    for java, bedrock in JAVA_TO_BEDROCK.items():
        assert "minecraft:" + bedrock in names, f"{java} -> {bedrock}: target not a Bedrock item"
    return names


def to_bedrock(java_item, names):
    """-> Bedrock name without namespace, or '' when no confirmed name exists."""
    java_item = java_item.replace("minecraft:", "")
    if java_item in COLLISIONS or "minecraft:" + java_item not in names:
        return JAVA_TO_BEDROCK.get(java_item, "")
    return java_item
