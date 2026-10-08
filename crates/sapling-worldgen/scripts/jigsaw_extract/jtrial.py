"""jtrial.py <start> <end>: jemu over BDS's trial chamber pool registration (build 1.26.52.3: 1481318c0-14813b822)
and print its pools as JSON lines: name, fallback and (template location, processor list, weight) items.

Elements are std::function factories built in place: 145e1afa0 (location, the shared empty processor list) and
145e1abc0 (location, a processor list by id, the id made by 141689de0 from namespace and path); pools are
145e231e0 (rcx out, r8 and r9 string views of name and fallback, stack[0] an initializer list of 0x48-byte
entries: the factory, then its weight at +0x40). jdata.py-style lines come from jjson.py's writer."""
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from jemu import Emu, core  # noqa: E402

ENTRY = 0x48


def view(e, a):
    """A {ptr, len} string view stored at a."""
    ptr = e.read(a, 8)
    if isinstance(ptr, tuple) and ptr[0] == 'cstr':
        return ptr[1]
    if isinstance(ptr, tuple) and ptr[0] == 'va':
        ptr = ptr[1]
    n = e.read(Emu.shift(a, 8), 8)
    if isinstance(ptr, int) and isinstance(n, int) and 0 < n < 300:
        return core.cstr(ptr, n)[:n]
    return None


def element(e, at, a):
    if isinstance(a['rcx'], tuple):
        e.objs[a['rcx']] = ('elem', view(e, a['rdx']), None)


def element_procs(e, at, a):
    procs = e.objs.get(a['r8'])
    if isinstance(a['rcx'], tuple):
        e.objs[a['rcx']] = ('elem', view(e, a['rdx']), procs[1] if isinstance(procs, tuple) and procs[0] == 'rl' else None)


def resource_location(e, at, a):
    if isinstance(a['rcx'], tuple):
        e.objs[a['rcx']] = ('rl', f"{view(e, a['r8'])}:{view(e, a['rdx'])}")


def pool(e, at, a):
    lst = a['stack'][0]
    begin, end = e.read(lst, 8), e.read(Emu.shift(lst, 8), 8)
    items = []
    if isinstance(begin, tuple) and isinstance(end, tuple) and begin[0] == end[0] == 'S':
        for off in range(begin[1], end[1], ENTRY):
            el = e.objs.get(('S', off))
            weight = e.read(('S', off + 0x40), 4)
            items.append([el[1], el[2], weight] if isinstance(el, tuple) and el[0] == 'elem' else [str(el), None, weight])
    print(json.dumps({'pool': view(e, a['r8']), 'fallback': view(e, a['r9']), 'at': f'{at:x}', 'items': items}))


HANDLERS = {0x145e1afa0: element, 0x145e1abc0: element_procs, 0x141689de0: resource_location, 0x145e231e0: pool}
IGNORED = {0x14a18efa0, 0x14087da20, 0x141f89b50, 0x141f90be0, 0x14a0f9b60}

if __name__ == '__main__':
    emu = Emu()
    emu.run(int(sys.argv[1], 16), int(sys.argv[2], 16), HANDLERS)
    for ev in emu.events:
        if int(ev['call'], 16) not in IGNORED:
            print(json.dumps(ev))
