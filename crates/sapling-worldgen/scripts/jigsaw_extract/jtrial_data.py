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
# 141fa20f0: the structure's pool alias bindings in order (Java's): a random group (ranged and slow ranged take
# the same mob), then two random picks. The targets' templates differ (webs, bones, podzol around the spawner).
BINDINGS = [
    [[('ranged', f'ranged/{mob}'), ('slow_ranged', f'slow_ranged/{mob}')] for mob in ('skeleton', 'stray', 'poison_skeleton')],
    [[('melee', f'melee/{mob}')] for mob in ('zombie', 'husk', 'spider')],
    [[('small_melee', f'small_melee/{mob}')] for mob in ('slime', 'cave_spider', 'silverfish', 'baby_zombie')],
]


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
    spawner = 'trial_chambers/spawner/'
    for choices in BINDINGS:
        for _, target in (pair for pairs in choices for pair in pairs):
            elements.setdefault(spawner + target, f'j {spawner}{target} - 0 0 -')
            pools[spawner + target] = ('empty', [(spawner + target, 1)])
    aliases['trial_chambers'] = [[[(f'{spawner}contents/{a}', spawner + t) for a, t in pairs] for pairs in choices] for choices in BINDINGS]
