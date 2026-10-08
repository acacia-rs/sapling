"""JSON block references as the Rust blocks of src/block/table.rs, for features_data.py and trees_data.py."""
import os
import re

# Old names and states BDS flattens into the blocks of the table.
ALIASES = {('stone', 'andesite'): 'andesite', ('stone', 'diorite'): 'diorite', ('stone', 'granite'): 'granite',
           ('tallgrass', 'tall'): 'short_grass', ('tallgrass', 'default'): 'short_grass', ('tallgrass', 'fern'): 'fern'}


class NotPorted(Exception):
    pass


def _rust_blocks():
    """(name, states) -> variant, from the `blocks!` table."""
    found = {}
    for table in ('table.rs', 'cave_table.rs', 'plant_table.rs'):
        path = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'src', 'block', table)
        text = open(path, encoding='utf-8').read()
        # The first variant of a (name, states) pair wins: the waterlogged twins come later.
        for m in re.finditer(r'^\s*(\w+) => "([a-z_]+)" "([^"]*)",', text, re.M):
            found.setdefault((m[2], m[3]), m[1])
    return found


BLOCKS = _rust_blocks()


def _states(text):
    return dict(p.split('=', 1) for p in text.split(',') if p)


def block(value):
    """The Rust block for a JSON block reference: the variant whose states include the given ones (the
    first of the name when none are given); NotPorted when the table lacks it."""
    name, states = (value['name'], value.get('states') or {}) if isinstance(value, dict) else (value, {})
    name = name.removeprefix('minecraft:')
    flattened = next((v for k, v in states.items() if k in ('stone_type', 'tall_grass_type')), None)
    name = ALIASES.get((name, flattened), name)
    wanted = {k: str(int(v) if isinstance(v, bool) else v) for k, v in states.items() if k not in ('stone_type', 'tall_grass_type')}
    for attempt in (wanted, _cardinal(wanted)):
        for (n, text), variant in BLOCKS.items():
            if n == name and attempt.items() <= _states(text).items():
                return 'Block::' + variant
    raise NotPorted(f'block {name} {wanted}')


def _cardinal(states):
    """The old integer `direction` (0 south, 1 west, 2 north, 3 east) as `minecraft:cardinal_direction`."""
    if 'direction' not in states:
        return states
    out = {k: v for k, v in states.items() if k != 'direction'}
    out['minecraft:cardinal_direction'] = ['south', 'west', 'north', 'east'][int(states['direction'])]
    return out


def blocks(values):
    """A block list that may name blocks sapling never generates (dropped: nothing can match them)."""
    out = []
    for value in values if isinstance(values, list) else [values]:
        try:
            out.append(block(value))
        except NotPorted:
            pass
    return '&[' + ', '.join(out) + ']'
