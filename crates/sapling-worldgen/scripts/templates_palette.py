"""The Bedrock side of templates_to_rust.py: each template state's block per rotation, and the palette of
Bedrock blocks (name, states, waterlogged, flags) templates and structure code place.

Flags: 1 full cube, 2 collides (from the block registry, per probed state), 4 waterloggable (a flooded
probe: scripts/data/waterloggable.txt). The palette also holds what structure code makes of placed
blocks: other woods (template/wood.rs WOOD_FAMILIES), waterlogged twins, and every neighbour-shaped
variant (fence, pane and wall connections, stair corners) for template/connect.rs.
"""
import itertools
import os
import re
import struct
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
TEMPLATE_SRC = os.path.join(HERE, '..', 'src', 'structure', 'template')
JIGSAW, DATA = 'minecraft:jigsaw', 'minecraft:structure_block'
AIR = ('air', '', False, 0)
WATERLOGGABLE = 4
# Blocks the structure code places itself (template::named), flags as in the probe table.
EXTRA = [
    ('chest', 'minecraft:cardinal_direction=east', False, 2),
    ('chest', 'minecraft:cardinal_direction=east', True, 2),
    ('stone_bricks', '', False, 3),
    ('suspicious_gravel', 'brushed_progress=0,hanging=1', False, 3),
    ('suspicious_sand', 'brushed_progress=0,hanging=1', False, 3),
    # Ruined portal ageing (src/structure/ruined_portal.rs).
    ('cracked_stone_bricks', '', False, 3),
    ('mossy_stone_bricks', '', False, 3),
    ('crying_obsidian', '', False, 3),
    ('magma', '', False, 3),
    ('netherrack', '', False, 3),
    ('normal_stone_slab', 'minecraft:vertical_half=bottom', False, 2),
    ('stone_brick_slab', 'minecraft:vertical_half=bottom', False, 2),
    ('mossy_stone_brick_slab', 'minecraft:vertical_half=bottom', False, 2),
    ('mossy_stone_brick_wall', ','.join(f'wall_connection_type_{d}=none' for d in ('east', 'north', 'south', 'west'))
     + ',wall_post_bit=0', False, 2),
    # Jigsaw pool processor outputs (src/structure/jigsaw/pools.txt).
    ('cracked_deepslate_bricks', '', False, 3),
    ('cracked_deepslate_tiles', '', False, 3),
    ('mossy_cobblestone', '', False, 3),
    ('web', '', False, 0),
    ('brown_stained_glass_pane', '', False, 2),
    ('grass_block', '', False, 3),
    ('oak_planks', '', False, 3),
    ('spruce_planks', '', False, 3),
    ('acacia_planks', '', False, 3),
    ('deepslate', 'pillar_axis=y', False, 3),
    ('red_sandstone', '', False, 3),
] + [(crop, 'growth=7', False, 0) for crop in ('wheat', 'potatoes', 'carrots', 'beetroot')] + [
    (f'waxed_{age}_copper_bulb', 'lit=1,powered_bit=0', False, 3) for age in ('oxidized', 'weathered', 'exposed')] + [
    # Mansion chest markers and entrance stairs (src/structure/mansion/piece.rs).
    ('chest', f'minecraft:cardinal_direction={d}', False, 2) for d in ('north', 'south', 'west')] + [(stairs, f'minecraft:corner=none,upside_down_bit={up},weirdo_direction={w}', False, 2)
     for stairs in ('stone_brick_stairs', 'mossy_stone_brick_stairs', 'stone_stairs') for up in range(2) for w in range(4)]
CONNECTIONS = [f'minecraft:connection_{d}' for d in ('east', 'north', 'south', 'west')]
WALL_SIDES = [f'wall_connection_type_{d}' for d in ('east', 'north', 'south', 'west')]
SHAPED = {
    'minecraft:connection_east': [{k: v for k, v in zip(CONNECTIONS, bits)} for bits in itertools.product('01', repeat=4)],
    'wall_post_bit': [{**{k: v for k, v in zip(WALL_SIDES, sides)}, 'wall_post_bit': post}
                      for sides in itertools.product(('none', 'short', 'tall'), repeat=4) for post in '01'],
    'minecraft:corner': [{'minecraft:corner': c} for c in ('none', 'inner_left', 'inner_right', 'outer_left', 'outer_right')],
}


def wood_families():
    """WOOD_FAMILIES of src/structure/template/wood.rs."""
    text = open(os.path.join(TEMPLATE_SRC, 'wood.rs')).read()
    table = text.split('WOOD_FAMILIES')[1].split('];\n')[0]
    return [re.findall(r'"(\w+)"', row) for row in table.split('\n') if '"' in row]


def fluid(key):
    m = re.fullmatch(r'minecraft:(water|lava)\[level=(\d+)\]', key)
    return (m.group(1), f'liquid_depth={min(int(m.group(2)), 8)}', False, 0) if m else None


def state_table(keys, table):
    """The state table: u16 count; per state u8 kind (0 block, 1 jigsaw, 2 structure block), then the
    palette index placed at each clockwise rotation. Returns it and the palette dict."""
    palette = {AIR: 0}
    out, missing = bytearray(struct.pack('<H', len(keys))), []
    for key in sorted(keys, key=keys.get):
        kind = 1 if key.startswith(JIGSAW) else 2 if key.startswith(DATA) else 0
        out += bytes([kind])
        for rot in range(4):
            got = AIR if kind else fluid(key) or table.get((key, rot))
            if got is None:
                missing.append((key, rot))
                got = AIR
            out += struct.pack('<H', palette.setdefault(got, len(palette)))
    if missing:
        print(f'{len(missing)} states without a probed block, placed as air: {missing[:8]}', file=sys.stderr)
    return out, expand(palette)


def expand(palette):
    for extra in EXTRA:
        palette.setdefault(extra, len(palette))
    for (name, states, wl, flags) in list(palette):
        for family in wood_families():
            if name in family:
                for other in family:
                    palette.setdefault((other, states, wl, flags), len(palette))
    for (name, states, wl, flags) in list(palette):
        pairs = dict(p.split('=', 1) for p in states.split(',') if p)
        for marker, variants in SHAPED.items():
            if marker in pairs:
                for variant in variants:
                    shaped = ','.join(f'{k}={v}' for k, v in sorted({**pairs, **variant}.items()))
                    palette.setdefault((name, shaped, wl, flags), len(palette))
    waterloggable = {line.strip() for line in open(os.path.join(HERE, 'data', 'waterloggable.txt'))}
    flagged = {}
    for (name, states, wl, flags), i in palette.items():
        flagged[(name, states, wl, flags | (WATERLOGGABLE if name in waterloggable else 0))] = i
    for (name, states, wl, flags) in list(flagged):
        if flags & WATERLOGGABLE:
            flagged.setdefault((name, states, True, flags), len(flagged))
    return flagged


def palette_bytes(palette):
    """u16 count; per entry u8 name length, name, u16 states length, states, u8 waterlogged, u8 flags."""
    out = bytearray(struct.pack('<H', len(palette)))
    for (name, states, wl, flags) in sorted(palette, key=palette.get):
        out += bytes([len(name)]) + name.encode() + struct.pack('<H', len(states)) + states.encode() + bytes([wl, flags])
    return out
