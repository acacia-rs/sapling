"""Breakdown of the oracle's feature residue from `SAPLING_ORACLE_DUMP` files (tests/oracle_support/dump.rs).

usage: py -3 feature_residue.py <dump dir> [world prefix] [--cells <family>]

Per world: stable feature errors by family and kind (ours-extra: BDS kept the block from before
features; bds-only: our features left it; both: both changed it, differently), split into
  b  thread-local generator in BDS (glow lichen and sculk vein faces, and sculk walks they steer),
  c  within 8 blocks of a structure's area (* = a kind sapling does not place),
  d  the rest, deterministic as far as the -run2 twin shows,
  e  as d, in a chunk on the region's edge (ring chunks BDS decorates differently, see worldgen-bds-features.md);
plus (a), cells the twin disagrees on (not counted as wrong by the oracle).
"""

import collections
import os
import re
import sys

THREAD_LOCAL = {'glow_lichen', 'sculk_vein'}
SCULK = {'sculk', 'sculk_sensor', 'sculk_catalyst', 'sculk_shrieker'}
GROUPS = {
    'coral': ('coral', 'sea_pickle'),
    'dripstone': ('dripstone',),
    'sulfur spikes': ('sulfur_spike', 'potent_sulfur'),
    'trees': ('_log', '_leaves', 'propagule', 'mangrove_roots', 'vine', 'cocoa', 'bee_nest', 'creaking'),
    'plants': ('grass', 'fern', 'flower', 'tulip', 'poppy', 'dandelion', 'bush', 'petals', 'litter', 'eyeblossom',
               'pale_moss', 'pale_hanging', 'sunflower', 'lilac', 'rose', 'peony', 'orchid', 'bluet', 'daisy',
               'allium', 'lily_of', 'reeds', 'cactus', 'mushroom', 'waterlily', 'berry', 'pumpkin', 'melon',
               'bamboo', 'podzol', 'wildflowers'),
    'aquatic': ('seagrass', 'kelp'),
    'lush caves': ('moss', 'azalea', 'cave_vines', 'dripleaf', 'spore', 'roots', 'clay'),
    'ice': ('ice', 'snow'),
    'liquid': ('water', 'lava', 'bubble', 'magma', 'obsidian'),
}


def name(cell):
    return cell.split('[', 1)[0]


def group(block):
    if block in THREAD_LOCAL:
        return block
    if block in SCULK:
        return 'sculk'
    for g, parts in GROUPS.items():
        if any(p in block for p in parts):
            return g
    return block.removeprefix('deepslate_')


def classify(row):
    kind = 'bds-only' if row['ours'] == row['before'] else 'ours-extra' if row['bds'] == row['before'] else 'both'
    block = name(row['ours'] if kind == 'ours-extra' else row['bds'])
    if kind == 'both' and name(row['bds']) in ('air', 'water'):
        block = name(row['ours'])
    fam = group(block)
    if fam in THREAD_LOCAL or fam == 'sculk' or name(row['ours']) in THREAD_LOCAL:
        cat = 'b'
    elif row['structure']:
        cat = 'c'
    else:
        cat = 'd'
    return kind, fam, cat


def region_bounds():
    """Region chunk bounds from tests/oracle.rs, by the dump's file-name form of the region name."""
    source = open(os.path.join(os.path.dirname(__file__), '..', 'tests', 'oracle.rs')).read()
    found = re.findall(r'Region::new\("([^"]+)", (-?\d+), (-?\d+), (-?\d+), (-?\d+)\)', source)
    return {re.sub(r'[^A-Za-z0-9]', '_', name): tuple(map(int, rest)) for name, *rest in found}


def on_edge(bounds, row):
    if bounds is None:
        return False
    cx, cz = int(row['x']) // 16, int(row['z']) // 16
    x0, z0, x1, z1 = bounds
    return cx in (x0, x1) or cz in (z0, z1)


def rows(path):
    with open(path) as f:
        head = f.readline().rstrip('\n').split('\t')
        for line in f:
            values = line.rstrip('\n').split('\t')
            yield dict(zip(head, values + [''] * (len(head) - len(values))))


def main():
    root, prefix = sys.argv[1], sys.argv[2] if len(sys.argv) > 2 and not sys.argv[2].startswith('--') else ''
    want = sys.argv[sys.argv.index('--cells') + 1] if '--cells' in sys.argv else None
    by_region = '--by-region' in sys.argv
    worlds = collections.defaultdict(list)
    for file in sorted(os.listdir(root)):
        if file.endswith('.tsv') and file.startswith(prefix):
            worlds[file.split('.', 1)[0]].append(os.path.join(root, file))
    # Worlds of one or two regions (near origin, far out) are reported per region.
    for world in [w for w, files in worlds.items() if by_region or len(files) <= 2]:
        for file in worlds.pop(world):
            worlds[os.path.basename(file).rsplit('.', 1)[0]] = [file]
    for world, files in worlds.items():
        table = collections.Counter()
        cats = collections.Counter()
        unstable = collections.Counter()
        structures = collections.Counter()
        for file in files:
            region = os.path.basename(file).split('.', 1)[1].rsplit('.', 1)[0]
            for row in rows(file):
                kind, fam, cat = classify(row)
                if cat == 'd' and on_edge(BOUNDS.get(region), row):
                    cat = 'e'
                if row['stable'] == 'U':
                    unstable[fam] += 1
                    continue
                table[(fam, kind, cat)] += 1
                cats[cat] += 1
                if cat == 'c':
                    structures[row['structure']] += 1
                if want and fam == want:
                    print(region, row['x'], row['y'], row['z'], kind, row['before'], row['ours'], row['bds'], row['structure'], sep='\t')
        print(f'== {world}: stable errors {sum(cats.values())}: ' + ', '.join(f'{c} {n}' for c, n in sorted(cats.items()))
              + f'; (a) unstable {sum(unstable.values())}')
        fams = collections.Counter()
        for (fam, _, _), n in table.items():
            fams[fam] += n
        for fam, total in fams.most_common(25):
            parts = ' '.join(f'{k}/{c}:{table[(fam, k, c)]}' for k in ('ours-extra', 'bds-only', 'both') for c in 'bcde' if table[(fam, k, c)])
            print(f'  {fam:>18} {total:>6}  {parts}  (a {unstable[fam]})')
        if structures:
            print('  structures (c): ' + ', '.join(f'{s} {n}' for s, n in structures.most_common(8)))


BOUNDS = region_bounds()
main()
