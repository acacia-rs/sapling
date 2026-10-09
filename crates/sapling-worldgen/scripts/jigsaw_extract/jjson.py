"""jjson.py <bds> <out pools_v2.txt> <trial chambers jsonl>: the v2 (data-driven) jigsaw pools and processor
lists of BDS's vanilla behaviour packs (worldgen/template_pools*, worldgen/processors), newest pack winning, plus
the trial chambers' (jtrial.py's output), in jdata.py's format. Elements are named by their template and marked
`j` (placed as Java does); a capped processor becomes `c <processors> <limit> <rule>;...`."""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from bds_packs import _version, brarchive, jsonc  # noqa: E402
import jtrial_data as jtrial  # noqa: E402

# Pools of the structures a world without experiments generates.
STRUCTURES = ['trail_ruins', 'abandoned_camp']
# Pre-flattening states the packs still use, as BDS upgrades them.
UPGRADES = {('dirt', 'dirt_type=coarse'): 'coarse_dirt', ('suspicious_gravel', ''): 'suspicious_gravel[brushed_progress=0,hanging=1]'}


def archives(bds, prefix):
    """Documents of every worldgen brarchive under `prefix`, newest pack last."""
    packs_dir = os.path.join(bds, 'behavior_packs')
    packs = sorted((p for p in os.listdir(packs_dir) if p == 'vanilla' or p.startswith('vanilla_')), key=_version)
    for pack in packs:
        base = os.path.join(packs_dir, pack, '__brarchive', 'worldgen')
        for dirpath, _, names in os.walk(base):
            for n in sorted(names):
                path = os.path.join(dirpath, n)
                if os.path.relpath(path, base).replace('\\', '/').startswith(prefix):
                    for _, raw in brarchive(path):
                        yield jsonc(raw)


def short(name):
    return name.removeprefix('minecraft:')


def element_name(e):
    return e['location'] if 'location' in e else 'feature:' + short(e['feature'])


def state(out):
    name = short(out['name'])
    states = ','.join(f'{k}={v}' for k, v in sorted(out.get('states', {}).items()))
    name = UPGRADES.get((name, states), name if not states else f'{name}[{states}]')
    return name


def pred(p):
    kind = short(p['predicate_type'])
    if kind == 'always_true':
        return '*'
    if kind == 'block_match':
        return f"b={short(p['block'])}"
    if kind == 'random_block_match':
        return f"rb={short(p['block'])}@{p['probability']}"
    raise ValueError(p)


def rules(proc):
    assert short(proc['processor_type']) == 'rule', proc
    return [f"{pred(r['input_predicate'])}|{pred(r.get('location_predicate', {'predicate_type': 'always_true'}))}|{state(r['output_state'])}"
            for r in proc['rules']]


def pack_data(bds, procs, elements, pools):
    for doc in archives(bds, 'processors'):
        d = doc['minecraft:processor_list']
        lines = []
        for p in d['processors']:
            if short(p['processor_type']) == 'capped':
                lines.append(('c', p['limit'], rules(p['delegate'])))
            else:
                lines.append(('r', None, rules(p)))
        procs[short(d['description']['identifier'])] = lines
    for doc in archives(bds, 'template_pools'):
        d = doc['minecraft:template_pool']
        name = short(d['description']['identifier'])
        if name.split('/')[0] not in STRUCTURES:
            continue
        for item in d['elements']:
            e = item['element']
            if short(e['element_type']) == 'feature_pool_element':
                elements[element_name(e)] = f"f {short(e['feature'])}"
                continue
            assert short(e['element_type']) == 'single_pool_element', e
            proc = short(e['processors']) if isinstance(e.get('processors'), str) else '-'
            projection = 1 if e.get('projection') == 'terrain_matching' else 0
            elements[element_name(e)] = f"j {e['location']} {proc} {projection} 0 -"
        pools[name] = (d.get('fallback'), [(element_name(i['element']), i['weight']) for i in d['elements']])


def structure_data(bds):
    """`s` lines for the structure sets of several jigsaw structures (abandoned camps), in the set's order:
    set, structure, weight, start pool, max depth, horizontal reach, padding, the biome ids its filter accepts."""
    from bds_packs import IDS, biomes

    def accepts(tests, have):
        """A list of `has_biome_tag` tests, all of which must hold (the camps' filters)."""
        return all((t['value'] in have) == (t.get('operator', '==') in ('==', 'equals')) for t in tests)
    tags ={IDS[n]: set(d['minecraft:biome']['components'].get('minecraft:tags', {}).get('tags', []))
            for n, d in biomes(bds).items() if n in IDS}
    structures = {}
    for doc in archives(bds, 'structures'):
        if 'minecraft:jigsaw' in doc:
            d = doc['minecraft:jigsaw']
            structures[short(d['description']['identifier'])] = d
    lines = []
    for doc in archives(bds, 'structure_sets'):
        d = doc['minecraft:structure_set']
        entries = d['structures']
        if len(entries) < 2:
            continue
        for e in entries:
            s = structures[short(e['structure'])]
            ids = ','.join(str(i) for i in sorted(tags) if accepts(s.get('biome_filters'), tags[i])) or '-'
            lines.append(f"s {short(d['description']['identifier'])} {short(e['structure'])} {e['weight']} "
                         f"{short(s['start_pool'])} {s['max_depth']} {s['max_distance_from_center']['horizontal']} "
                         f"{s.get('dimension_padding', 0)} {ids}")
    return lines


def write(out_path, procs, elements, pools, aliases, structures=()):
    with open(out_path, 'w', newline='\n') as out:
        out.write('# BDS v2 jigsaw data (behaviour pack worldgen JSON, trial chambers from C++); scripts/jigsaw_extract/jjson.py\n')
        for name, lines in procs.items():
            plain = [r for kind, _, rs in lines if kind == 'r' for r in rs]
            out.write(f"r {name} {';'.join(plain) or '-'}\n")
            for kind, limit, rs in lines:
                if kind == 'c':
                    out.write(f"c {name} {limit} {';'.join(rs)}\n")
        for name, d in elements.items():
            out.write(f'e {name} {d}\n')
        for name, (fallback, items) in pools.items():
            fb = short(fallback) if fallback and short(fallback) != 'empty' else '-'
            out.write(f"p {name} {fb} " + ' '.join(f"{e}*{w}" for e, w in items) + '\n')
        # One `b` line per alias binding, in order: its choices (one is drawn), each a list of alias=target.
        for name, bindings in aliases.items():
            for choices in bindings:
                out.write(f"b {name} " + '|'.join(','.join(f'{a}={b}' for a, b in pairs) for pairs in choices) + '\n')
        for line in structures:
            out.write(line + '\n')
    print(len(procs), 'processor lists', len(elements), 'elements', len(pools), 'pools', len(aliases), 'alias sets')


def main(bds, out_path, trial_jsonl):
    procs, elements, pools, aliases = {}, {}, {}, {}
    pack_data(bds, procs, elements, pools)
    jtrial.load(trial_jsonl, procs, elements, pools, aliases)
    write(out_path, procs, elements, pools, aliases, structure_data(bds))


if __name__ == '__main__':
    main(*sys.argv[1:])
