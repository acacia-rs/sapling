"""parse_probe.py <probe log> <states.tsv> <out.tsv>: per Java state and rotation (0..3 clockwise) the Bedrock block BDS
placed: 'java<TAB>rot<TAB>name<TAB>states<TAB>waterlogged'. Only cells in chunks where the probe template was placed
count (a chunk is placed when several of its cells hold blocks the terrain never has). Reports disagreements."""
import collections
import json
import os
import re
import sys

ROT = {(1, 0, 0, 1): 0, (0, 1, -1, 0): 1, (-1, 0, 0, -1): 2, (0, -1, 1, 0): 3}
NATURAL = {'air', 'snow_layer', 'stone', 'dirt', 'grass_block', 'spruce_leaves', 'spruce_log', 'snow', 'water', 'ice',
           'packed_ice', 'gravel', 'sand', 'tall_grass', 'short_grass', 'fern', 'large_fern', 'sweet_berry_bush',
           'powder_snow', 'oak_leaves', 'oak_log', 'birch_leaves', 'birch_log', 'podzol', 'coarse_dirt', 'andesite',
           'granite', 'diorite', 'coal_ore', 'iron_ore', 'copper_ore', 'tuff', 'dandelion', 'poppy', 'deadbush',
           'sandstone', 'clay', 'red_sand', 'mud', 'cherry_leaves'}
java = {}
pos = {}
for line in open(sys.argv[2]):
    i, x, y, z, k = line.rstrip('\n').split('\t')
    java[int(i)] = k
    pos[int(i)] = (int(x), int(y), int(z))


REG = {}
FLAGS = {}
for line in open(os.environ['PROBE_REGISTRY']):  # registry.py's output
    n, props, fl = line.rstrip('\n').split('\t')
    REG.setdefault(n.replace('minecraft:', ''), {p.split('=')[0] for p in props.split(',') if p})
    FLAGS[(n.replace('minecraft:', ''), props)] = fl
CURRENT_NAME = [None]


def states_text(js):
    keep = REG.get(CURRENT_NAME[0])
    if keep is not None:
        js = {k: v for k, v in js.items() if k in keep}

    def val(v):
        if v is True:
            return '1'
        if v is False:
            return '0'
        return str(v)
    return ','.join(f'{k}={val(v)}' for k, v in sorted(js.items()))


cell = re.compile(r'PROBE cell (-?\d+),(-?\d+),(-?\d+),(-?\d+) (\d+) (\S+) (\{.*\}|undefined) (true|false|undefined)')
markers = re.compile(r'PROBE temple (-?\d+) (-?\d+) markers \[\[\{"x":(-?\d+),"y":(-?\d+),"z":(-?\d+)\}\]')
igloos = []
for line in open(sys.argv[1], encoding='utf-8', errors='replace'):
    m = markers.search(line)
    if m:
        igloos.append({'sponge': tuple(int(m.group(i)) for i in (3, 4, 5)), 'cells': []})
        continue
    m = cell.search(line)
    if m:
        ex, ez = (int(m.group(1)), int(m.group(2))), (int(m.group(3)), int(m.group(4)))
        idx = int(m.group(5))
        name = m.group(6).replace('minecraft:', '')
        CURRENT_NAME[0] = name
        st = states_text(json.loads(m.group(7))) if m.group(7) != 'undefined' else ''
        sx, sy, sz = igloos[-1]['sponge']
        x, y, z = pos[idx]
        wx, wz = sx + (x - 1) * ex[0] + (z - 1) * ez[0], sz + (x - 1) * ex[1] + (z - 1) * ez[1]
        igloos[-1]['rot'] = ROT[ex + ez]
        igloos[-1]['cells'].append((idx, (wx >> 4, wz >> 4), (name, st, m.group(8) == 'true')))

seen = collections.defaultdict(collections.Counter)
for ig in igloos:
    unnatural = collections.Counter(ch for _, ch, (name, _, _) in ig['cells'] if name not in NATURAL)
    placed = {ch for ch, n in unnatural.items() if n >= 3}
    for idx, ch, obs in ig['cells']:
        if ch in placed:
            seen[(java[idx], ig['rot'])][obs] += 1

with open(sys.argv[3], 'w') as f:
    for (k, rot), c in sorted(seen.items()):
        (name, st, wl), n = c.most_common(1)[0]
        if len(c) > 1:
            print('disagree', k, rot, dict(c))
        f.write(f'{k}\t{rot}\t{name}\t{st}\t{int(wl)}\t{FLAGS.get((name, st), "?")}\n')
missing = [(k, r) for k in set(java.values()) for r in range(4) if (k, r) not in seen]
print('igloos', len(igloos), 'covered', len(seen), 'missing', len(missing), missing[:10])
