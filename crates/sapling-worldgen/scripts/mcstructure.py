"""Bedrock .mcstructure templates (little-endian NBT) in the shape templates_to_rust.py reads Java .nbt ones: size,
palette of {Name, Properties}, blocks of {pos, state, nbt}. Only what the bastion's mob pieces use: one block
layer, jigsaws (their block entity's name, target, target_pool, joint, final_state), plain blocks; structure
voids (index -1) are left out."""
import struct

# A Bedrock jigsaw's `rotation` for a vertical facing is its top direction.
FACING = ['down', 'up', 'north', 'south', 'west', 'east']
TOPS = ['north', 'east', 'south', 'west']


def _read(d, i, t):
    simple = {1: '<b', 2: '<h', 3: '<i', 4: '<q', 5: '<f', 6: '<d'}
    if t in simple:
        return struct.unpack_from(simple[t], d, i)[0], i + struct.calcsize(simple[t])
    if t == 7:
        n = struct.unpack_from('<i', d, i)[0]
        return d[i + 4:i + 4 + n], i + 4 + n
    if t == 8:
        n = struct.unpack_from('<H', d, i)[0]
        return d[i + 2:i + 2 + n].decode(), i + 2 + n
    if t == 9:
        et, n = d[i], struct.unpack_from('<i', d, i + 1)[0]
        i += 5
        out = []
        for _ in range(n):
            v, i = _read(d, i, et)
            out.append(v)
        return out, i
    if t == 10:
        out = {}
        while (tt := d[i]) != 0:
            n = struct.unpack_from('<H', d, i + 1)[0]
            key = d[i + 3:i + 3 + n].decode()
            out[key], i = _read(d, i + 3 + n, tt)
        return out, i + 1
    if t in (11, 12):
        n = struct.unpack_from('<i', d, i)[0]
        w = 4 if t == 11 else 8
        return list(struct.unpack_from(f'<{n}{"i" if t == 11 else "q"}', d, i + 4)), i + 4 + w * n
    raise ValueError(t)


def _java_state(entry):
    name, states = entry['name'], entry.get('states', {})
    if name != 'minecraft:jigsaw':
        assert not states, f'unhandled Bedrock states {entry}'
        return {'Name': name}
    facing = FACING[states['facing_direction']]
    top = TOPS[states.get('rotation', 0)] if facing in ('down', 'up') else 'up'
    return {'Name': name, 'Properties': {'orientation': f'{facing}_{top}'}}


def load(path):
    d = open(path, 'rb').read()
    n = struct.unpack_from('<H', d, 1)[0]
    root, _ = _read(d, 3 + n, 10)
    sx, sy, sz = root['size']
    s = root['structure']
    pal = s['palette']['default']
    entities = pal.get('block_position_data', {})
    blocks = []
    for index, state in enumerate(s['block_indices'][0]):
        if state < 0:
            continue
        x, y, z = index // (sy * sz), index // sz % sy, index % sz
        block = {'pos': [x, y, z], 'state': state}
        data = entities.get(str(index), {}).get('block_entity_data')
        if data and data.get('id') == 'JigsawBlock':
            block['nbt'] = {'name': data['name'], 'target': data['target'], 'pool': data['target_pool'],
                            'joint': data['joint'], 'final_state': data['final_state']}
        blocks.append(block)
    # DataVersion past Java's flattening: not a legacy template.
    return {'size': [sx, sy, sz], 'palette': [_java_state(e) for e in pal['block_palette']], 'blocks': blocks, 'DataVersion': 9999}
