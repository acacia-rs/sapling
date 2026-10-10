"""dbg_vein_cmp.py <feature_trace log> <dbg.log> [rule substring] [-v]: per try of a multiface rule (default
sculk_vein_feature), our placement and spread against BDS's `mfset` and `spset` hits of the same call (the
dbg_sculk.py config). Says what BDS's thread-local face order changed: nothing, the face, the cell, the
spread's cell, or whether anything was placed at all."""
import re
import sys
from collections import Counter

args = [a for a in sys.argv[1:] if not a.startswith('-')]
rule = args[2] if len(args) > 2 else 'sculk_vein_feature'
verbose = '-v' in sys.argv

ours = []  # (pos, [(cell, block)])
cur = None
for line in open(args[0]):
    if line.startswith(' '):
        m = re.match(r'\s+write (-?\d+) (-?\d+) (-?\d+) (\S+) over', line)
        if m and cur is not None:
            cur[1].append((tuple(map(int, m.groups()[:3])), m.group(4)))
        continue
    m = re.search(rule + r' try \d+ at \[(-?\d+), (-?\d+), (-?\d+)\] placed', line)
    cur = (tuple(map(int, m.groups())), []) if m else None
    if cur:
        ours.append(cur)

threads = {}
for line in open(args[1]):
    m = re.match(r'hit (\w+) (t\d+) pos=\((-?\d+), (-?\d+), (-?\d+)\) match=True(.*)', line)
    if m and m.group(1) in ('mf', 'mfset', 'spset', 'patch'):
        bits = re.search(r'\]=\((\d+),\)', m.group(6))
        threads.setdefault(m.group(2), []).append((m.group(1), tuple(map(int, m.groups()[2:5])), int(bits.group(1)) if bits else 0))
want = {pos for pos, _ in ours}
t = max(threads, key=lambda t: sum(1 for name, pos, _ in threads[t] if name == 'mf' and pos in want))
calls, cur = [], None  # (pos, [(name, cell, bits)])
for name, pos, bits in threads[t]:
    if name == 'mf':
        cur = (pos, [])
        calls.append(cur)
    elif name == 'patch':
        cur = None
    elif cur is not None:
        cur[1].append((name, pos, bits))

tally, k = Counter(), 0
for pos, writes in ours:
    while k < len(calls) and calls[k][0] != pos:
        k += 1
    if k == len(calls):
        print(f'our try at {pos} is not in thread {t}')
        break
    hits = calls[k][1]
    k += 1
    placed = [(cell, bits) for name, cell, bits in hits if name == 'mfset']
    spread = [(cell, bits) for name, cell, bits in hits if name == 'spset']
    first, rest = (writes[0][0] if writes else None), [cell for cell, _ in writes[1:]]
    if not placed and first is None:
        kind = 'neither places'
    elif not placed or first is None:
        kind = 'only BDS places' if placed else 'only ours places'
    elif placed[0][0] != first:
        kind = 'another cell'
    else:
        kind = 'same cell'
    tally[kind] += 1
    if placed and first is not None:
        ours_spread, bds_spread = (rest[0] if rest else None), (spread[0][0] if spread else None)
        tally['spread: ' + ('neither' if not rest and not spread else 'BDS only' if not rest else 'ours only' if not spread else 'same cell' if ours_spread == bds_spread else 'another cell')] += 1
    if verbose or kind not in ('neither places', 'same cell') or (placed and bool(rest) != bool(spread)):
        print(f'{pos}: ours {writes}   bds placed {placed} spread {spread}   [{kind}]')
print(f'thread {t}: {len(ours)} tries of ours, {len(calls)} multiface calls')
for kind, n in sorted(tally.items()):
    print(f'  {kind}: {n}')
