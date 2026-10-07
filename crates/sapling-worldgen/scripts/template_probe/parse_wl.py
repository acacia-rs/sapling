"""parse_wl.py <probe log> <states.tsv> <out.tsv>: from a shipwreck probe (flooded placement), per Bedrock block
(name, canonical states) whether BDS waterlogged it below sea level: 'name<TAB>states<TAB>waterlogged<TAB>count'."""
import collections
import json
import os
import re
import sys

REG = {}
for line in open(os.environ['PROBE_REGISTRY']):  # registry.py's output
    n, props, fl = line.rstrip('\n').split('\t')
    REG.setdefault(n.replace('minecraft:', ''), {p.split('=')[0] for p in props.split(',') if p})

pos = {}
for line in open(sys.argv[2]):
    i, x, y, z, k = line.rstrip('\n').split('\t')
    pos[int(i)] = (int(x), int(y), int(z))


def states_text(name, js):
    keep = REG.get(name)
    if keep is not None:
        js = {k: v for k, v in js.items() if k in keep}
    val = lambda v: '1' if v is True else '0' if v is False else str(v)
    return ','.join(f'{k}={val(v)}' for k, v in sorted(js.items()))


cell = re.compile(r'PROBE cell (-?\d+),(-?\d+),(-?\d+),(-?\d+) (\d+) (\S+) (\{.*\}|undefined) (true|false|undefined)')
markers = re.compile(r'PROBE temple (-?\d+) (-?\d+) markers \[\[\{"x":(-?\d+),"y":(-?\d+),"z":(-?\d+)\}\]')
seen = collections.defaultdict(collections.Counter)
sponge_y = None
for line in open(sys.argv[1], encoding='utf-8', errors='replace'):
    m = markers.search(line)
    if m:
        sponge_y = int(m.group(4))
        continue
    m = cell.search(line)
    if m and m.group(7) != 'undefined':
        y = sponge_y + pos[int(m.group(5))][1] - 1
        if y >= 62:
            continue
        name = m.group(6).replace('minecraft:', '')
        seen[(name, states_text(name, json.loads(m.group(7))))][m.group(8) == 'true'] += 1

with open(sys.argv[3], 'w') as f:
    for (name, st), c in sorted(seen.items()):
        wl, n = c.most_common(1)[0]
        f.write(f'{name}\t{st}\t{int(wl)}\t{n}\t{dict(c)}\n')
