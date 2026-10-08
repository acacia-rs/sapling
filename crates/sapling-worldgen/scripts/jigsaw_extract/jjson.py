"""jjson.py <bds> <out pools_v2.txt>: the v2 (data-driven) jigsaw pools and processor lists of BDS's vanilla
behaviour packs (worldgen/template_pools*, worldgen/processors), newest pack winning, in jdata.py's format.
Elements are named by their template and marked `j` (placed as Java does); a capped processor becomes
`c <processors> <limit> <rule>;...`."""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
from bds_packs import _version, brarchive, jsonc  # noqa: E402

# Pools of the structures a world without experiments generates.
STRUCTURES = ['trail_ruins']
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


def main(bds, out_path):
    procs, pools = {}, {}
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
        if name.split('/')[0] in STRUCTURES:
            pools[name] = (d.get('fallback'), d['elements'])
    elements = {}
    with open(out_path, 'w', newline='\n') as out:
        out.write('# BDS v2 jigsaw data (behaviour pack worldgen JSON); scripts/jigsaw_extract/jjson.py\n')
        for name, lines in procs.items():
            plain = [r for kind, _, rs in lines if kind == 'r' for r in rs]
            out.write(f"r {name} {';'.join(plain) or '-'}\n")
            for kind, limit, rs in lines:
                if kind == 'c':
                    out.write(f"c {name} {limit} {';'.join(rs)}\n")
        for _, items in pools.values():
            for item in items:
                e = item['element']
                if short(e['element_type']) == 'feature_pool_element':
                    elements[element_name(e)] = f"f {short(e['feature'])}"
                    continue
                assert short(e['element_type']) == 'single_pool_element', e
                proc = short(e['processors']) if isinstance(e.get('processors'), str) else '-'
                projection = 1 if e.get('projection') == 'terrain_matching' else 0
                elements[element_name(e)] = f"j {e['location']} {proc} {projection} 0 -"
        for name, d in elements.items():
            out.write(f'e {name} {d}\n')
        for name, (fallback, items) in pools.items():
            fb = short(fallback) if fallback and short(fallback) != 'empty' else '-'
            out.write(f"p {name} {fb} " + ' '.join(f"{element_name(i['element'])}*{i['weight']}" for i in items) + '\n')
    print(len(procs), 'processor lists', len(elements), 'elements', len(pools), 'pools')


if __name__ == '__main__':
    main(*sys.argv[1:])
