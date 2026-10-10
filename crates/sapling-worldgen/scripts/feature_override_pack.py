"""Writes a pack overlay that makes BDS place one feature wherever it places another, to check a feature no
oracle world holds (docs/research/worldgen-bds-trees.md, "Checking a feature no world places").

    py -3 -I scripts/feature_override_pack.py <bds> <out dir> <feature to test> <identifier to take over>
    EXTRA_PACK=<out dir> scripts/bds_dbg_run.sh ...        then examples/swap_check on the saved world

Names are given without `minecraft:` (orange_poplar_tree_feature spruce_tree_feature).
"""
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from bds_packs import documents  # noqa: E402

bds, out, source, target = sys.argv[1:5]
doc = documents(bds, 'features')[source + '.json']
kind = next(k for k in doc if k.startswith('minecraft:'))
doc[kind]['description']['identifier'] = 'minecraft:' + target
os.makedirs(os.path.join(out, 'features'), exist_ok=True)
with open(os.path.join(out, 'features', target + '.json'), 'w', encoding='utf-8') as f:
    json.dump(doc, f, indent=1)
print(f'{target} carries the {kind} of {source} ({doc["format_version"]})')
