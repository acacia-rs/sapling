"""jpools.py <start> <end> ...: run jemu over registration functions and print elements, pools, alias sets
and processor lists as JSON lines. BLOCK_NAMES: a file of '0x<global va> minecraft:<name>' lines naming
the VanillaBlockTypeIds globals (FNV-1 64 of the name, set by their initialiser).

Build 1.26.52.3: villages 148140520-148159d7c (processors), 14816cc00-1481b0721 (elements),
1480bbe90-1480e53c4 (pools); ancient cities 148109380-14810ee76 (elements), 1480fe430-1481046e5
(processors, pools). jdata.py turns the output into src/structure/jigsaw/pools.txt."""
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from jemu import Emu  # noqa: E402

records = []


def show(e, v):
    if isinstance(v, tuple) and v[0] in ('S', 'H'):
        s = e.string_at(v)
        if s is not None:
            return {'str': s}
        d = e.read(v, 8)
        if isinstance(d, tuple):
            return show(e, d)
        b = e.read(v, 1)
        return {'byte': b, 'q': d if isinstance(d, int) else None}
    if isinstance(v, tuple):
        return list(v) if v[0] != 'bytes' else 'bytes'
    return v


def lookup(kind):
    def h(e, at, a):
        return (kind, e.arg_str(a['rdx']))
    return h


def factory(name, stack_args):
    def h(e, at, a):
        rec = {'at': f'{at:x}', 'factory': name, 'location': e.arg_str(a['r8']), 'r9': show(e, a['r9']),
               'stack': [show(e, a['stack'][k]) for k in range(stack_args)]}
        records.append(rec)
        e.write(a['rcx'] if a['rcx'] is None else (a['rcx'] if a['rcx'][0] == 'S' else a['rcx']), 8, ('rec', len(records) - 1))
        return None
    return h


def ctor(e, at, a):
    rec = {'at': f'{at:x}', 'factory': 'ctor145e18510', 'location': e.arg_str(a['r8']), 'r9': show(e, a['r9']),
           'stack': [show(e, ('S', 0x20 + 8 * k - e.rbp_off)) for k in range(4)]}
    records.append(rec)
    if isinstance(a['rcx'], tuple):
        e.objs[('tag',) + tuple(a['rcx'])] = len(records) - 1
    return None


def register_element(e, at, a):
    slot = a['r8'] if isinstance(a['r8'], tuple) and a['r8'][0] == 'rec' else e.deref(a['r8'])
    idx = None
    if isinstance(slot, tuple) and slot[0] == 'rec':
        idx = slot[1]
    elif isinstance(slot, tuple) and slot[0] == 'H':
        idx = e.objs.get(('tag',) + tuple(slot))
    print(json.dumps({'element': e.arg_str(a['rdx']), 'def': records[idx] if idx is not None else show(e, a['r8'])}))


def feature(name):
    def h(e, at, a):
        rec = {'at': f'{at:x}', 'factory': name, 'rdx': show(e, a['rdx']), 'r8': show(e, a['r8']), 'r9': show(e, a['r9'])}
        records.append(rec)
        print(json.dumps({'feature_rec': len(records) - 1, 'def': rec}))
        if isinstance(a['rcx'], tuple):
            e.write(a['rcx'], 8, ('rec', len(records) - 1))
        return None
    return h


def pool(name_reg, fallback_reg, vec_reg):
    def h(e, at, a):
        vec = a[vec_reg]
        items = e.vectors.pop(Emu.shift(vec, 8), []) if isinstance(vec, tuple) and vec[0] == 'S' else []
        out = {'pool': e.arg_str(a[name_reg]), 'fallback': e.arg_str(a[fallback_reg]) if fallback_reg else None, 'at': f'{at:x}',
               'items': [[v[1] if isinstance(v, tuple) else str(v), w] for v, w in items]}
        print(json.dumps(out))
        return None
    return h


def other(e, at, a):
    print(json.dumps({'call_at': f'{at:x}', 'rdx': show(e, a['rdx']), 'r8': show(e, a['r8']), 'r9': show(e, a['r9'])}))


HANDLERS = {
    0x141f7dc50: lookup('proc'),
    0x141f80c00: lookup('elemref'),
    0x141f7f8b0: register_element,
    0x1481d8670: factory('8670', 0),
    0x1481d8940: factory('8940', 0),
    0x1481d8c00: factory('8c00', 2),
    0x1481d8f00: factory('8f00', 1),
    0x1481d91f0: factory('91f0', 2),
    0x1481d9500: factory('9500', 1),
    0x1481110c0: factory('110c0', 2),
    0x145e18510: ctor,
    0x145e137d0: feature('137d0'),
    0x1481113c0: feature('113c0'),
    0x1481d97f0: pool('rdx', 'r8', 'r9'),
    0x148111680: pool('rdx', None, 'r8'),
}



BLOCKS = {}
for line in open(os.environ['BLOCK_NAMES']):
    a, n = line.split()
    BLOCKS[int(a, 16)] = n.replace('minecraft:', '')
VTABLES = {0x14a89aea0: 'random_block_match', 0x14a89ada0: 'always_true', 0x14a89ae20: 'list_pred'}


def block_lookup(e, at, a):
    rdx = a['rdx']
    if isinstance(rdx, tuple) and rdx[0] in ('va', 'cstr'):
        return ('block', BLOCKS.get(rdx[1], f'?{rdx[1]:x}') if rdx[0] == 'va' else '?' + rdx[1])
    return ('block', '?')


def heap_objs_since(e, start):
    return sorted({k[1] for k in list(e.objs) + list(e.mem) if k[0] == 'H' and k[1] > start})


def decode_pred(e, p):
    if not (isinstance(p, tuple) and p[0] == 'H'):
        return p
    vt = e.objs.get(p)
    name = VTABLES.get(vt[1], hex(vt[1])) if isinstance(vt, tuple) and vt[0] == 'va' else str(vt)
    fields = []
    for off in (8, 0x10, 0x18):
        v = e.objs.get(Emu.shift(p, off))
        if v is None:
            v = e.read(Emu.shift(p, off), 4)
            if isinstance(v, int):
                import struct
                v = round(struct.unpack('<f', struct.pack('<I', v))[0], 4)
        fields.append(v if not isinstance(v, tuple) else (v[1] if v[0] in ('block', 'list') else list(v)))
    return [name] + fields


LAST = [0]


def register_list(e, at, a):
    rules = []
    lst = e.deref(a['r8'])
    ptrs = [v for v, _ in e.vectors.get(Emu.shift(lst, 8), [])] if isinstance(lst, tuple) and lst[0] == 'H' else []
    for base in ptrs:
        if not (isinstance(base, tuple) and base[0] == 'H'):
            continue
        out = e.objs.get(Emu.shift(base, 0x18))
        if isinstance(out, tuple) and out[0] == 'block':
            rules.append({'input': decode_pred(e, e.objs.get(base)), 'location': decode_pred(e, e.objs.get(Emu.shift(base, 8))),
                          'position': decode_pred(e, e.objs.get(Emu.shift(base, 0x10))), 'output': out[1]})
    LAST[0] = e.heap
    print(json.dumps({'processors': e.arg_str(a['rdx']), 'at': f'{at:x}', 'rules': rules}))


HANDLERS[0x1419be4c0] = block_lookup
HANDLERS[0x141f7e0e0] = register_list



ALIASES = {'from': None, 'pairs': []}


def alias_from(e, at, a):
    ALIASES['from'] = e.arg_str(a['r8'])
    return None


def alias_to(e, at, a):
    ALIASES['pairs'].append([ALIASES['from'], e.arg_str(a['r8'])])
    return None


def alias_set(e, at, a):
    print(json.dumps({'aliases': e.arg_str(a['rdx']), 'pairs': ALIASES['pairs']}))
    ALIASES['pairs'] = []


HANDLERS[0x14816c9a0] = alias_from
HANDLERS[0x14816c770] = alias_to
HANDLERS[0x141f7f500] = alias_set


HANDLERS[0x145e22c70] = pool('rdx', 'r8', 'r9')


def block_match_ctor(e, at, a):
    obj = a['rcx']
    if isinstance(obj, tuple) and obj[0] == 'H':
        e.objs[obj] = ('va', 0x14a89ae60)
        e.objs[Emu.shift(obj, 8)] = a['rdx']
    return None


HANDLERS[0x145e158f0] = block_match_ctor
VTABLES[0x14a89ae60] = 'block_match'
VTABLES[0x14a89ade0] = 'list_random'
VTABLES[0x14a89ae20] = 'not_in_list'


BLOCKS[0x14ccf9950] = 'air'


def block_list(e, at, a):
    begin, end = e.objs.get(a['rdx']), e.objs.get(Emu.shift(a['rdx'], 8))
    names = []
    if isinstance(begin, tuple) and isinstance(end, tuple) and begin[0] == 'S':
        for off in range(begin[1], end[1], 8):
            v = e.objs.get(('S', off))
            names.append(v[1] if isinstance(v, tuple) and v[0] == 'block' else str(v))
    if isinstance(a['rcx'], tuple):
        e.objs[a['rcx']] = ('list', tuple(names))
    return None


def list_pred_ctor(e, at, a):
    if isinstance(a['rcx'], tuple):
        e.objs[a['rcx']] = e.objs.get(a['rdx'])
    return None


HANDLERS[0x148109130] = block_list
HANDLERS[0x145e2b030] = list_pred_ctor


def ctor_plain(e, at, a):
    rec = {'at': f'{at:x}', 'factory': 'ctor145e132d0', 'location': e.arg_str(a['r8']), 'r9': a['r9'] if isinstance(a['r9'], int) else show(e, a['r9']),
           'stack': [show(e, ('S', 0x20 - e.rbp_off))]}
    records.append(rec)
    if isinstance(a['rcx'], tuple):
        e.objs[('tag',) + tuple(a['rcx'])] = len(records) - 1
    return None


HANDLERS[0x145e132d0] = ctor_plain


HANDLERS[0x141f80e80] = lambda e, at, a: ('elemref', 'minecraft:empty')


def vector_emplace(e, at, a):
    vec = a['rcx']
    if isinstance(vec, tuple) and vec[0] == 'S':
        value, weight = e.deref(a['r8']), e.read(a['r9'], 4) if isinstance(a['r9'], tuple) else None
        e.vectors.setdefault(Emu.shift(vec, 8), []).append([value, weight])
    return None


HANDLERS[0x141f80770] = lambda e, at, a: ('elemref', 'minecraft:empty')
HANDLERS[0x142f73470] = vector_emplace


if __name__ == '__main__':
    args = sys.argv[1:]
    for k in range(0, len(args), 2):
        e = Emu()
        e.run(int(args[k], 16), int(args[k + 1], 16), HANDLERS)
        for ev in e.events:
            print(json.dumps(ev))
