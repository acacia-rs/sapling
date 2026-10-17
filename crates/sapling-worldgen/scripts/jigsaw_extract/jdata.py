"""jdata.py <out pools.txt> <jsonl>...: turns jpools.py output into the jigsaw data file sapling loads.

Lines: `l <list> <block>,...` block lists; `r <processors> <rule>;...` with rule `in|location|output` or
`in|location|lp=<axis>:<min>:<max>:<min d>:<max d>|output` (a linear position test), a predicate being `*`, `b=<block>`, `rb=<block>@<p>`, `nl=<list>` or `lr=<list>@<p>`; `e <element> t <template>
<processors|-> <projection> <post> <aliases|->` template elements, `e <element> f <feature>` feature elements;
`p <pool> <fallback|-> <element>*<weight> ...`; `a <alias set> <from>=<to> ...`."""
import json
import sys

# The farm lists' crop rules (BlockMatch -> the crop at growth 7, its state set in code, so jpools.py
# misses them), read off the disassembly; they come first in each list.
CROPS = {
    'wheat_wheat_farm': [('carrots', 'wheat')],
    'wheat_potato_farm': [('carrots', 'potatoes')],
    'wheat_beet_farm': [('carrots', 'beetroot')],
    'potato_potato_farm': [('wheat', 'potatoes'), ('carrots', 'potatoes')],
    'potato_beet_farm': [('wheat', 'potatoes'), ('carrots', 'beetroot')],
    'potato_carrot_farm': [('wheat', 'potatoes')],
    'beet_beet_farm': [('wheat', 'beetroot'), ('carrots', 'beetroot')],
    'beet_carrot_farm': [('wheat', 'beetroot')],
    'carrot_carrot_farm': [('wheat', 'carrots')],
}

# Axis-aligned linear position tests (vtable 14a89af30) by processor list: jpools.py does not read the test's
# fields, so they are the constructor's arguments at 148113af9 (min, max chance, min, max distance, axis Y).
POSITION_TESTS = {'bastion_treasure_room_highrampart': 'lp=y:0.0:0.05:0:100'}

lists, procs, elements, pools, aliases, feature_recs = {}, {}, {}, [], [], {}


def rule_text(processors, r):
    if r['position'][0] == 'always_true':
        return f"{pred(r['input'])}|{pred(r['location'])}|{r['output']}"
    return f"{pred(r['input'])}|{pred(r['location'])}|{POSITION_TESTS[short(processors)]}|{r['output']}"


def list_name(blocks):
    key = tuple(blocks)
    for name, value in lists.items():
        if value == key:
            return name
    name = f'list{len(lists)}'
    lists[name] = key
    return name


def pred(p):
    kind = p[0]
    if kind == 'always_true':
        return '*'
    if kind == 'block_match':
        return f'b={p[1]}'
    if kind == 'random_block_match':
        return f'rb={p[1]}@{p[2]}'
    if kind == 'not_in_list':
        return f'nl={list_name(p[1])}'
    if kind == 'list_random':
        return f'lr={list_name(p[1])}@{p[3]}'
    raise ValueError(p)


def short(name):
    return name.replace('minecraft:', '') if name else '-'


for path in sys.argv[2:]:
    for line in open(path, encoding='utf-8'):
        if not line.startswith('{'):
            continue
        d = json.loads(line)
        if 'processors' in d:
            rules = [rule_text(d['processors'], r) for r in d['rules']]
            crops = CROPS.get(short(d['processors']).removeprefix('abandoned_'), [])
            procs[d['processors']] = [f'b={a}|*|{b}[growth=7]' for a, b in crops] + rules
        elif 'feature_rec' in d:
            feature_recs[d['feature_rec']] = d['def']
        elif 'element' in d and d['element']:
            df = d['def']
            if isinstance(df, list) and df[0] == 'rec':
                df = feature_recs[df[1]]
            f = df.get('factory')
            if f in ('137d0', '113c0'):
                feature = df['r9'][1] if isinstance(df['r9'], list) else df['r9'].get('str')
                elements[d['element']] = f'f {short(feature)}'
                continue
            proc = df['r9'][1] if isinstance(df['r9'], list) and df['r9'][0] == 'proc' else None
            stack = df.get('stack') or []
            alias = None
            projection, post = 0, 3
            if f == '8f00':
                projection = stack[0]['byte']
            elif f == '91f0':
                alias, projection = stack[0][1], stack[1]['byte']
            elif f in ('9500', '8c00'):
                alias = stack[0][1]
            elif f == '110c0':
                projection, post = stack[0]['byte'], stack[1]['byte']
            elif f == 'ctor145e18510':
                alias = stack[0][1] if isinstance(stack[0], list) else None
                projection, post = stack[2]['byte'], stack[3]['byte']
            elif f == 'ctor145e132d0':
                proc, projection, post = None, df['r9'], stack[0]['byte']
            elif f == '8940':
                proc = None
            elements[d['element']] = f"t {df['location']} {short(proc)} {projection} {post} {short(alias)}"
        elif 'pool' in d:
            items = ' '.join(f'{short(n)}*{w}' for n, w in d['items'])
            pools.append(f"p {short(d['pool'])} {short(d['fallback'])} {items}")
        elif 'aliases' in d:
            aliases.append(f"a {short(d['aliases'])} " + ' '.join(f'{short(a)}={short(b)}' for a, b in d['pairs']))

with open(sys.argv[1], 'w', newline='\n') as out:
    out.write('# BDS hardcoded jigsaw data (village, ancient city); docs/research/worldgen-bds-jigsaw.md\n')
    for name, blocks in lists.items():
        out.write(f"l {name} {','.join(blocks)}\n")
    for name, rules in procs.items():
        out.write(f"r {short(name)} {';'.join(rules) if rules else '-'}\n")
    for name, d in elements.items():
        out.write(f'e {short(name)} {d}\n')
    for line in pools + aliases:
        out.write(line + '\n')
print(len(lists), 'lists', len(procs), 'processor lists', len(elements), 'elements', len(pools), 'pools', len(aliases), 'alias sets')
