"""Reading BDS's vanilla behaviour packs: brarchives in version order, JSON with comments, biome ids."""
import json
import os
import re
import struct

# Bedrock biome ids of the biomes the overworld tree yields (src/biome/ids.rs).
IDS = {
    'ocean': 0, 'plains': 1, 'desert': 2, 'extreme_hills': 3, 'forest': 4, 'taiga': 5, 'swampland': 6,
    'river': 7, 'frozen_river': 11, 'ice_plains': 12, 'mushroom_island': 14, 'beach': 16, 'jungle': 21,
    'jungle_edge': 23, 'deep_ocean': 24, 'stone_beach': 25, 'cold_beach': 26, 'birch_forest': 27,
    'roofed_forest': 29, 'cold_taiga': 30, 'mega_taiga': 32, 'extreme_hills_plus_trees': 34, 'savanna': 35,
    'savanna_plateau': 36, 'mesa': 37, 'mesa_plateau_stone': 38, 'warm_ocean': 40, 'deep_warm_ocean': 41,
    'lukewarm_ocean': 42, 'deep_lukewarm_ocean': 43, 'cold_ocean': 44, 'deep_cold_ocean': 45,
    'frozen_ocean': 46, 'deep_frozen_ocean': 47, 'bamboo_jungle': 48, 'sunflower_plains': 129,
    'extreme_hills_mutated': 131, 'flower_forest': 132, 'ice_plains_spikes': 140, 'birch_forest_mutated': 155,
    'redwood_taiga_mutated': 160, 'savanna_mutated': 163, 'mesa_bryce': 165, 'jagged_peaks': 182,
    'frozen_peaks': 183, 'snowy_slopes': 184, 'grove': 185, 'meadow': 186, 'lush_caves': 187,
    'dripstone_caves': 188, 'stony_peaks': 189, 'deep_dark': 190, 'mangrove_swamp': 191, 'cherry_grove': 192,
    'pale_garden': 193, 'sulfur_caves': 194,
}
# The Nether's (src/nether/biome.rs).
NETHER_IDS = {'hell': 8, 'soulsand_valley': 178, 'crimson_forest': 179, 'warped_forest': 180, 'basalt_deltas': 181}


def brarchive(path):
    data = open(path, 'rb').read()
    count, = struct.unpack_from('<I', data, 8)
    base = 16 + count * 256
    for i in range(count):
        entry = 16 + i * 256
        name = data[entry + 1:entry + 1 + data[entry]].decode()
        offset, length = struct.unpack_from('<II', data, entry + 248)
        yield name, data[base + offset:base + offset + length]


def _version(pack):
    tail = pack.removeprefix('vanilla').lstrip('_')
    return tuple(int(p) for p in tail.split('.')) if tail else ()


def jsonc(text):
    """JSON that may hold // and /* */ comments and trailing commas."""
    if isinstance(text, bytes):
        text = text.decode('utf-8-sig')
    out, i = [], 0
    while i < len(text):
        if text[i] == '"':
            j = i + 1
            while text[j] != '"':
                j += 2 if text[j] == '\\' else 1
            out.append(text[i:j + 1])
            i = j + 1
        elif text.startswith('//', i):
            i = text.find('\n', i) if '\n' in text[i:] else len(text)
        elif text.startswith('/*', i):
            i = text.find('*/', i) + 2
        else:
            out.append(text[i])
            i += 1
    return json.loads(re.sub(r',(\s*[}\]])', r'\1', ''.join(out)))


def documents(bds, kind):
    """`kind` documents ('biomes', 'features', 'feature_rules') by file name: `definitions/<kind>` first, then
    each `vanilla*` pack in version order, a later file replacing an earlier one of the same name, as a world on
    the newest base game version sees them."""
    found = {}
    loose = os.path.join(bds, 'definitions', kind)
    if os.path.isdir(loose):
        for name in sorted(os.listdir(loose)):
            found[name] = jsonc(open(os.path.join(loose, name), 'rb').read())
    packs_dir = os.path.join(bds, 'behavior_packs')
    packs = sorted((p for p in os.listdir(packs_dir) if p == 'vanilla' or p.startswith('vanilla_')), key=_version)
    for pack in packs:
        path = os.path.join(packs_dir, pack, '__brarchive', kind + '.brarchive')
        if os.path.exists(path):
            for name, raw in brarchive(path):
                found[name] = jsonc(raw)
    return found


def load_order(bds, kind):
    """`kind` documents in the order BDS loads them: each `vanilla*` pack newest first in brarchive entry order,
    then `definitions/<kind>` by name."""
    packs_dir = os.path.join(bds, 'behavior_packs')
    packs = sorted((p for p in os.listdir(packs_dir) if p == 'vanilla' or p.startswith('vanilla_')), key=_version, reverse=True)
    for pack in packs:
        path = os.path.join(packs_dir, pack, '__brarchive', kind + '.brarchive')
        if os.path.exists(path):
            for _, raw in brarchive(path):
                yield jsonc(raw)
    loose = os.path.join(bds, 'definitions', kind)
    if os.path.isdir(loose):
        for name in sorted(os.listdir(loose)):
            yield jsonc(open(os.path.join(loose, name), 'rb').read())


def biomes(bds):
    return {name.removesuffix('.biome.json'): doc for name, doc in documents(bds, 'biomes').items()}
