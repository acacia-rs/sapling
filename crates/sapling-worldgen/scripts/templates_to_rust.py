"""Transcribes BDS's structure templates (Java .nbt files in the vanilla behaviour packs, the families in
FAMILIES) and the Bedrock blocks they place into src/structure/template/templates.bin.

    py -3 -I scripts/templates_to_rust.py <bds>

A Java state becomes, per rotation, the Bedrock block in scripts/data/template_blocks.tsv: what BDS placed
for it, read off probe worlds (scripts/template_probe). Fluids are converted as BDS's converter does
(`level` to `liquid_depth`); jigsaws and structure blocks keep their own kinds for the placement code.

templates.bin, little-endian: b'SPT1', the palette (templates_palette.py), u32 count; per template: u8 name length, name, u16 x3 size,
u8 legacy (saved before Java's flattening), u32 block count, then per block u8 x, u16 y, u8 z, u16 state (an index into the state table); then u16
special count, per special: u32 block index, u8 kind (0 jigsaw: name, target, pool, joint, orientation
strings, u16 final state, i32 selection and placement priorities; 1 data marker: metadata string; 2 chest
whose block entity draws a loot seed; 3 vine with Java's `up`), strings as u8 length + bytes; then the state table
(templates_palette.py).
"""
import gzip
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from templates_palette import DATA, JIGSAW, palette_bytes, state_table  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, '..', 'src', 'structure', 'template')
FAMILIES = ['igloo', 'shipwreck', 'ruin', 'ruined_portal', 'village', 'pillageroutpost', 'ancient_city',
            'trial_chambers', 'trail_ruins', 'fossils', 'mansion', 'abandoned_camp', 'nether_fossils']


def version(pack):
    return (0,) if pack == 'vanilla' else tuple(int(p) for p in pack.split('_')[1].split('.'))


def effective(bds):
    """Template path (no extension) -> file, the newest pack winning."""
    root = os.path.join(bds, 'behavior_packs')
    files = {}
    for pack in sorted((p for p in os.listdir(root) if p == 'vanilla' or p.startswith('vanilla_')), key=version):
        base = os.path.join(root, pack, 'structures')
        for dirpath, _, names in os.walk(base):
            for n in names:
                if n.endswith('.nbt'):
                    full = os.path.join(dirpath, n)
                    files[os.path.relpath(full, base).replace('\\', '/')[:-4]] = full
    return {k: v for k, v in files.items() if k.split('/')[0] in FAMILIES}


class Nbt:
    """Big-endian Java NBT."""

    def __init__(self, data):
        self.d, self.i = data, 0

    def take(self, fmt):
        v = struct.unpack_from('>' + fmt, self.d, self.i)[0]
        self.i += struct.calcsize('>' + fmt)
        return v

    def string(self):
        n = self.take('H')
        self.i += n
        return self.d[self.i - n:self.i].decode('utf-8')

    def payload(self, t):
        simple = {1: 'b', 2: 'h', 3: 'i', 4: 'q', 5: 'f', 6: 'd'}
        if t in simple:
            return self.take(simple[t])
        if t == 7:
            n = self.take('i')
            self.i += n
            return self.d[self.i - n:self.i]
        if t == 8:
            return self.string()
        if t == 9:
            et, n = self.take('b'), self.take('i')
            return [self.payload(et) for _ in range(n)]
        if t == 10:
            out = {}
            while (et := self.take('b')) != 0:
                key = self.string()
                out[key] = self.payload(et)
            return out
        if t in (11, 12):
            n = self.take('i')
            return [self.take('i' if t == 11 else 'q') for _ in range(n)]
        raise ValueError(t)


def load(path):
    r = Nbt(gzip.decompress(open(path, 'rb').read()))
    assert r.take('b') == 10
    r.string()
    return r.payload(10)


def key_of(name, props):
    return name + ('[' + ','.join(f'{k}={v}' for k, v in sorted(props.items())) + ']' if props else '')


def parse_key(text):
    name, _, rest = text.partition('[')
    if ':' not in name:
        name = 'minecraft:' + name
    props = dict(p.split('=', 1) for p in rest.rstrip(']').split(',') if p)
    return key_of(name, props)


# Pre-1.16 jigsaws (villages): `facing` and one attachment name for both ends, as Java's datafixer reads them.
OLD_FACING = {'down': 'down_south', 'up': 'up_north'}


def jigsaw_strings(nbt, props):
    """name, target, pool, joint, orientation of a jigsaw block."""
    if 'attachement_type' in nbt:
        facing = props.get('facing', 'north')
        orientation = OLD_FACING.get(facing, f'{facing}_up')
        kind = nbt['attachement_type']
        return [kind, kind, nbt.get('target_pool', ''), '', orientation]
    return [nbt.get('name', ''), nbt.get('target', ''), nbt.get('pool', ''), nbt.get('joint', ''), props.get('orientation', 'north_up')]


def brewing_stand(key, nbt):
    """BDS sets a brewing stand's bottle bits from its items' raw slot numbers (Java 0-2 bottles, Bedrock
    1-3): the stand gets its own state key, the same block at every rotation."""
    if not key.startswith('minecraft:brewing_stand') or 'Items' not in nbt:
        return key, None
    slots = {item['Slot'] for item in nbt['Items']}
    states = ','.join(f'brewing_stand_slot_{c}_bit={int(s in slots)}' for c, s in zip('abc', (1, 2, 3)))
    return f'{key}#slots={sorted(slots)}', ('brewing_stand', states, False, 2)


def placed_wall(key, rot, probed):
    """A wall as the loader places it, before the shape pass the probe saw it after: its Java sides turned `rot`
    quarters clockwise (pre-1.16 boolean sides load as short), which the wall below reads (template/connect.rs)."""
    props = dict(p.split('=', 1) for p in key.partition('[')[2].rstrip(']').split(',') if p)
    turned = ['north', 'east', 'south', 'west']
    loaded = {'low': 'short', 'true': 'short', 'tall': 'tall'}
    sides = {turned[(i + rot) % 4]: loaded.get(props.get(side), 'none') for i, side in enumerate(turned)}
    states = ','.join(f'wall_connection_type_{side}={sides[side]}' for side in sorted(sides))
    return (probed[0], f"{states},wall_post_bit={int(props.get('up', 'true') == 'true')}", probed[2], probed[3])


def placed_sides(key, rot, probed):
    """A fence, pane or bars as the loader places it: its Java sides turned `rot` quarters clockwise. The probe's
    lone block had every side recomputed by the update pass (docs/research/worldgen-bds-templates.md); in a
    template the sides toward blocks it places keep these."""
    props = dict(p.split('=', 1) for p in key.partition('[')[2].rstrip(']').split(',') if p)
    turned = ['north', 'east', 'south', 'west']
    states = probed[1]
    for i, side in enumerate(turned):
        to = turned[(i + rot) % 4]
        states = states.replace(f'minecraft:connection_{to}=0', f"minecraft:connection_{to}={int(props.get(side) == 'true')}")
    return (probed[0], states, probed[2], probed[3])


def as_saved(key, rot, probed, table):
    """A circuit block or corner stair as its template saved it, where the probe's lone block had settled before
    it was read: a city's circuit holds its power (docs/research/worldgen-bds-templates.md, "Saved states")."""
    name, _, rest = key.partition('[')
    props = dict(p.split('=', 1) for p in rest.rstrip(']').split(',') if p)
    # A corner stair is placed as one (the probe's stood alone and were straight again when read): a rule
    # that wants the default state passes it over; template/connect.rs names the corner afterwards.
    if name.endswith('_stairs') and props.get('shape', 'straight') != 'straight':
        return (probed[0], probed[1].replace('minecraft:corner=none', f"minecraft:corner={props['shape']}"), probed[2], probed[3])
    if name == 'minecraft:redstone_wire':
        return (probed[0], f"redstone_signal={props['power']}", probed[2], probed[3])
    if name == 'minecraft:repeater' and props.get('powered') == 'true':
        return ('powered_repeater',) + probed[1:]
    if name == 'minecraft:redstone_lamp' and props.get('lit') == 'true':
        return ('lit_redstone_lamp',) + probed[1:]
    if name == 'minecraft:piston_head':
        base = 'sticky_piston' if props.get('type') == 'sticky' else 'piston'
        piston = table.get((f"minecraft:{base}[extended=true,facing={props['facing']}]", rot))
        return (base + '_arm_collision', piston[1], False, 0) if piston else probed
    return probed


def main():
    bds = sys.argv[1]
    table = {}
    for line in open(os.path.join(HERE, 'data', 'template_blocks.tsv')):
        java, rot, name, states, wl, flags = line.rstrip('\n').split('\t')
        table[(java, int(rot))] = (name, states, wl == '1', int(flags))
    templates = sorted(effective(bds).items())
    keys = {}

    def index(key):
        return keys.setdefault(key, len(keys))

    out = bytearray(b'SPT1') + struct.pack('<I', len(templates))
    for name, path in templates:
        t = load(path)
        palette = [key_of(s['Name'], s.get('Properties', {})) for s in t.get('palette') or t['palettes'][0]]
        sx, sy, sz = t['size']
        assert max(sx, sz) < 256 and sy < 65536, name
        # Saved before Java's flattening (DataVersion 1451; mansions carry none): the loader's old converter.
        legacy = t.get('DataVersion', 0) < 1451
        out += bytes([len(name)]) + name.encode() + struct.pack('<HHHBI', sx, sy, sz, legacy, len(t['blocks']))
        specials = []
        for i, b in enumerate(t['blocks']):
            x, y, z = b['pos']
            nbt = b.get('nbt', {})
            key, fixed = brewing_stand(palette[b['state']], nbt)
            if fixed:
                table.update({(key, rot): fixed for rot in range(4)})
            out += struct.pack('<BHBH', x, y, z, index(key))
            if key.startswith(JIGSAW):
                final = parse_key(nbt.get('final_state', 'minecraft:air'))
                props = (t.get('palette') or t['palettes'][0])[b['state']].get('Properties', {})
                strings = jigsaw_strings(nbt, props)
                priorities = (nbt.get('selection_priority', 0), nbt.get('placement_priority', 0))
                specials.append((i, 0, strings, (index(final), priorities)))
            elif key.startswith(DATA) and 'mode=data' in key:
                specials.append((i, 1, [nbt.get('metadata', '')], None))
            elif nbt.get('id') in ('minecraft:chest', 'minecraft:trapped_chest') and 'LootTableSeed' not in nbt:
                specials.append((i, 2, [], None))
            elif key.startswith('minecraft:vine[') and 'up=true' in key:
                specials.append((i, 3, [], None))
        out += struct.pack('<H', len(specials))
        for i, kind, strings, final in specials:
            out += struct.pack('<IB', i, kind)
            for s in strings:
                out += bytes([len(s.encode())]) + s.encode()
            if final is not None:
                out += struct.pack('<Hii', final[0], *final[1])
    for key in keys:
        if (key, 0) in table and 'wall_post_bit' in table[(key, 0)][1]:
            table.update({(key, rot): placed_wall(key, rot, table[(key, 0)]) for rot in range(4)})
        if (key, 0) in table and 'minecraft:connection_' in table[(key, 0)][1]:
            table.update({(key, rot): placed_sides(key, rot, table[(key, rot)]) for rot in range(4) if (key, rot) in table})
        table.update({(key, rot): as_saved(key, rot, table[(key, rot)], table) for rot in range(4) if (key, rot) in table})
    states, palette = state_table(keys, table)
    out = b'SPT1' + palette_bytes(palette) + out[4:] + states
    open(os.path.join(OUT, 'templates.bin'), 'wb').write(out)
    print(f'{len(templates)} templates, {len(keys)} states, {len(palette)} blocks, {len(out)} bytes')


if __name__ == '__main__':
    main()
