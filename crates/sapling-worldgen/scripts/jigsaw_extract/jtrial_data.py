"""The trial chambers' jigsaw data for jjson.py: jtrial.py's pools, plus what BDS builds around them, read off the
disassembly. Elements 145e1afa0 builds carry the copper bulb list; those 145e1abc0 builds (an id beside the
location) carry none."""
import json

COPPER = 'trial_chambers_copper_bulb_degradation'
# 1480ba050: three random_block_match rules on waxed copper bulbs, each output lit.
COPPER_RULES = [f'rb=waxed_copper_bulb@{p}|*|waxed_{age}_copper_bulb[lit=1,powered_bit=0]'
                for p, age in (('0.1', 'oxidized'), ('0.33333334', 'weathered'), ('0.5', 'exposed'))]
# The empty element's global (EmptyPoolElement).
EMPTY = "('va', 5545505424)"
# 141fa20f0: the structure's pool aliases, each alias's first target (a random pick in BDS: every target is one
# trial spawner, so the blocks agree whichever it draws).
ALIASES = {
    'trial_chambers/spawner/contents/ranged': ['ranged/skeleton', 'ranged/stray', 'ranged/poison_skeleton'],
    'trial_chambers/spawner/contents/slow_ranged': ['slow_ranged/skeleton', 'slow_ranged/stray', 'slow_ranged/poison_skeleton'],
    'trial_chambers/spawner/contents/melee': ['melee/zombie', 'melee/husk', 'melee/spider'],
    'trial_chambers/spawner/contents/small_melee': ['small_melee/slime', 'small_melee/cave_spider', 'small_melee/silverfish',
                                                    'small_melee/baby_zombie'],
}


def short(name):
    return name.removeprefix('minecraft:')


def load(path, procs, elements, pools, aliases):
    procs[COPPER] = [('r', None, COPPER_RULES)]
    for line in open(path):
        d = json.loads(line)
        if 'pool' not in d:
            continue
        items = []
        for location, plain, weight in d['items']:
            if location == EMPTY:
                items.append(('empty', weight))
                continue
            proc = '-' if plain else COPPER
            name = location if elements.get(location, f'j {location} {proc} 0 0 -').split()[2] == proc else f'{location}#plain'
            elements[name] = f'j {location} {proc} 0 0 -'
            items.append((name, weight))
        pools[short(d['pool'])] = (d['fallback'], items)
    pairs = []
    for alias, targets in ALIASES.items():
        target = 'trial_chambers/spawner/' + targets[0]
        elements.setdefault(target, f'j {target} - 0 0 -')
        pools[target] = ('empty', [(target, 1)])
        pairs.append((alias, target))
    aliases['trial_chambers'] = pairs
