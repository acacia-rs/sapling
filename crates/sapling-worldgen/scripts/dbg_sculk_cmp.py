"""dbg_sculk_cmp.py <feature_trace log> <dbg.log> [-v] [-t N]: per sculk patch try, our writes against the BDS thread's
writes between two patch entries. Prints per-try counts by kind and the first differing write of each try."""
import re
import sys
from collections import Counter

verbose = '-v' in sys.argv
only = int(sys.argv[sys.argv.index('-t') + 1]) if '-t' in sys.argv else None


def kind(block):
    if block.startswith('SculkVein'):
        return ('vein', int(block[9:].removeprefix('Wet')))
    return (block, 0)


ours = []  # (pos, [(kind, bits, pos)], label)
cur = None
for line in open(sys.argv[1]):
    if line.startswith(' '):
        m = re.match(r'\s+write (-?\d+) (-?\d+) (-?\d+) (\S+) over (\S+)', line)
        if m and cur is not None:
            k, bits = kind(m.group(4))
            cur[1].append((k, bits, tuple(map(int, m.groups()[:3]))))
        continue
    m = re.search(r'sculk_patch_feature try (\d+) at \[(-?\d+), (-?\d+), (-?\d+)\] placed (\w+) draws (\S+)', line)
    cur = None
    if m:
        cur = (tuple(map(int, m.groups()[1:4])), [], f'try {m.group(1)} placed {m.group(5)} draws {m.group(6)}')
        ours.append(cur)

NAMES = {'conv': 'Sculk', 'growth': 'growth', 'catalyst': 'SculkCatalyst', 'shrieker': 'SculkShrieker'}
threads = {}
for line in open(sys.argv[2]):
    m = re.match(r'hit (\w+) (t\d+) pos=\((-?\d+), (-?\d+), (-?\d+)\) match=True(.*)', line)
    if not m:
        continue
    name, t, pos, rest = m.group(1), m.group(2), tuple(map(int, m.groups()[2:5])), m.group(6)
    bits = re.search(r'\]=\((\d+),\)', rest)
    bt = re.search(r'bt=(\S+)', rest)
    threads.setdefault(t, []).append((name, pos, int(bits.group(1)) if bits else 0, bt.group(1) if bt else ''))
t = max(threads, key=lambda t: sum(1 for e in threads[t] if e[0] == 'patch'))
bds, cur = [], None
for name, pos, bits, bt in threads[t]:
    if name == 'patch':
        cur = (pos, [])
        bds.append(cur)
    elif name == 'mf':
        cur = None
    elif cur is not None and name != 'mfset' and name != 'spset':
        if name in ('saset', 'regrow', 'rmface'):
            cur[1].append(('vein' if bits else 'Air', bits, pos, name))
        else:
            cur[1].append((NAMES[name], 0, pos, name))
print(f'thread {t}: {len(bds)} patch entries; ours {len(ours)} tries')
tot_o, tot_b = Counter(), Counter()
shown = 0
for i, (pos, writes, label) in enumerate(ours):
    b = bds[i] if i < len(bds) else None
    if b is None or b[0] != pos:
        print(f'{i}: ours at {pos} {label}, bds {b and b[0]}: position mismatch')
        break
    ow = [(k if k not in ('SculkSensor', 'SculkShrieker') else 'growth', bits, p) for k, bits, p in writes]
    bw = [(k, bits, p) for k, bits, p, _ in b[1]]
    # the patch's own shriekers are logged as growth on neither side: keep names as they are
    co, cb = Counter(k for k, _, _ in ow), Counter(k for k, _, _ in bw)
    tot_o.update(co)
    tot_b.update(cb)
    same = [(k, p) for k, _, p in ow] == [(k, p) for k, _, p in bw]
    n = 0
    while n < len(ow) and n < len(bw) and (ow[n][0], ow[n][2]) == (bw[n][0], bw[n][2]):
        n += 1
    if not same:
        # What the two walks leave behind: cells whose last write differs, and how far from the first difference.
        last_o, last_b = {p: (k, bits) for k, bits, p in ow}, {p: (k, bits) for k, bits, p in bw}
        cells = [p for p in last_o.keys() | last_b.keys() if last_o.get(p) != last_b.get(p)]
        at = (ow[n] if n < len(ow) else bw[n])[2]
        far = max((max(abs(a - b) for a, b in zip(p, at)) for p in cells), default=0)
        print(f'{i}: {pos}: {len(cells)} cells end differently, up to {far} from the first difference at {at}')
    if (not same and shown < 12 and only is None) or only == i:
        shown += 1
        print(f'{i}: {pos} {label}: ours {dict(co)} bds {dict(cb)}; first {n} of {len(ow)}/{len(bw)} writes agree')
        for k in range(max(0, n - 3), min(max(len(ow), len(bw)), n + (40 if only == i else 6))):
            o = ow[k] if k < len(ow) else None
            x = b[1][k] if k < len(bw) else None
            print(f'      {k}: ours {o}   bds {x}')
    elif verbose:
        print(f'{i}: {pos} {label}: {"same" if same else "DIFF"} {len(ow)}/{len(bw)}')
print('totals ours', dict(tot_o))
print('totals bds ', dict(tot_b))
mf = Counter(e[0] for e in threads[t] if e[0] in ('mf', 'mfset', 'spset'))
print('bds multiface', dict(mf))
