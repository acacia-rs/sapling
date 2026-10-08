"""mkprobe.py <out.nbt> <states.tsv>: a Java template holding every distinct Java state of the template families
(jigsaw final states included; fluids, jigsaws and structure blocks left out), one per even cell, plus three markers
(sponge at (1,1,1), honeycomb_block at (3,1,1), dried_kelp_block at (1,1,3)). states.tsv lists index, x, y, z, state."""
import gzip
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scan import effective, load, state_key

FAMILIES = ['igloo', 'shipwreck', 'ruin', 'ruined_portal', 'village', 'pillageroutpost', 'ancient_city', 'trial_chambers',
            'trail_ruins', 'fossils', 'mansion']
SKIP = ('minecraft:jigsaw', 'minecraft:structure_block', 'minecraft:water', 'minecraft:lava')


def parse_state(text):
    if '[' not in text:
        return {'Name': text}
    name, rest = text.split('[', 1)
    props = dict(p.split('=', 1) for p in rest.rstrip(']').split(',') if p)
    return {'Name': name, 'Properties': props}


states = {}
for rel, full in effective().items():
    if rel.split('/')[0] not in FAMILIES:
        continue
    t = load(full)
    pal = t.get('palette') or t['palettes'][0]
    for b in t['blocks']:
        s = pal[b['state']]
        if not s['Name'].startswith(SKIP):
            states.setdefault(state_key(s), s)
        fs = b.get('nbt', {}).get('final_state')
        if fs:
            ps = parse_state(fs if ':' in fs.split('[')[0] else 'minecraft:' + fs)
            if not ps['Name'].startswith(SKIP):
                states.setdefault(state_key(ps), ps)

keys = sorted(states)
if len(sys.argv) > 3:
    only = {l.strip() for l in open(sys.argv[3]) if l.strip()}
    keys = [k for k in keys if k in only]
side = 1
while side ** 3 < len(keys):
    side += 1
size = 2 * side
# With a list of states, a short wide layout: x and z below 16 stay in the chunks BDS places for every rotation.
flat = len(sys.argv) > 3
if flat:
    side = int(os.environ.get('PROBE_SIDE', 8))
    size = 2 * side
palette = [states[k] for k in keys] + [{'Name': 'minecraft:sponge'}, {'Name': 'minecraft:honeycomb_block'},
                                        {'Name': 'minecraft:dried_kelp_block'}]
blocks = []
with open(sys.argv[2], 'w') as f:
    for i, k in enumerate(keys):
        if flat:
            x, y, z = 2 * (i % side), 2 * (i // side // side) + 2, 2 * (i // side % side)
        else:
            x, y, z = 2 * (i % side), 2 * (i // side % side) + 2, 2 * (i // side // side)
        blocks.append({'pos': [x, y, z], 'state': i})
        f.write(f'{i}\t{x}\t{y}\t{z}\t{k}\n')
n = len(keys)
blocks += [{'pos': [1, 1, 1], 'state': n}, {'pos': [3, 1, 1], 'state': n + 1}, {'pos': [1, 1, 3], 'state': n + 2}]


def w_str(s):
    b = s.encode()
    return struct.pack('>H', len(b)) + b


def w_tag(t, name, payload):
    return bytes([t]) + w_str(name) + payload


def compound(items):
    out = b''
    for k, (t, p) in items.items():
        out += w_tag(t, k, p)
    return out + b'\0'


def int_list(vals):
    return bytes([3]) + struct.pack('>i', len(vals)) + b''.join(struct.pack('>i', v) for v in vals)


def comp_list(comps):
    return bytes([10]) + struct.pack('>i', len(comps)) + b''.join(comps)


def state_nbt(s):
    items = {'Name': (8, w_str(s['Name']))}
    if s.get('Properties'):
        items['Properties'] = (10, compound({k: (8, w_str(v)) for k, v in s['Properties'].items()}))
    return compound(items)


root = compound({
    'size': (9, int_list([size, size + 2, size])),
    'palette': (9, comp_list([state_nbt(s) for s in palette])),
    'blocks': (9, comp_list([compound({'pos': (9, int_list(b['pos'])), 'state': (3, struct.pack('>i', b['state']))}) for b in blocks])),
    'entities': (9, bytes([10]) + struct.pack('>i', 0)),
    'DataVersion': (3, struct.pack('>i', 3937)),
})
open(sys.argv[1], 'wb').write(gzip.compress(bytes([10]) + w_str('') + root))
print(len(keys), 'states, size', size)
