"""Sets our feature calls of one source chunk beside one BDS thread's breakpoint hits.

usage: py -3 -I dbg_calls.py <feature_trace output> <dbg.log> <kind,kind...> [first]

<feature_trace output> is `examples/feature_trace <seed> <cx> <cz> [substring]`: a try's line, then its inner lines
`  <kind> x y z ...` (snap, pointed, cluster, large, grow with `facing f height h`, patch, cell, plant, roots). The
breakpoints must carry the same names and a position filter (bds_dbg.py logs `hit <name> t<id> pos=(x, y, z)`); grow
also logs r8 and r9 (facing, height). The thread holding our first three calls is taken from there on (or from our
call number [first]), and the first difference is printed with the rule try each of ours belongs to. Compare only
kinds whose rules run alone in BDS's log: a rule filtered out of our trace still shows in the thread.
"""
import re
import sys

CONTEXT = 14


def s32(v):
    v &= 0xFFFFFFFF
    return v - (1 << 32) if v >> 31 else v


def ours(path, kinds):
    calls, label = [], '?'
    for line in open(path):
        if line.startswith(' '):
            m = re.match(r'\s+(\w+) (-?\d+) (-?\d+) (-?\d+)(?: facing (\d) height (-?\d+))?', line)
            if m and m.group(1) in kinds:
                call = (m.group(1), tuple(map(int, m.groups()[1:4])))
                calls.append((call + ((int(m.group(5)), int(m.group(6))),) if m.group(5) else call, label))
            continue
        rule = re.search(r'(\S+) try (\d+) at (\[[^\]]*\]).*?(draws \S+)?$', line.rstrip())
        label = f'{rule.group(1).split(":")[-1]} try {rule.group(2)} at {rule.group(3)} {rule.group(4) or ""}' if rule else '?'
    return calls


def hits(path, kinds):
    threads = {}
    for line in open(path):
        m = re.match(r'hit (\w+) (t\d+) pos=\((-?\d+), (-?\d+), (-?\d+)\)', line)
        if m and m.group(1) in kinds:
            call = (m.group(1), tuple(map(int, m.groups()[2:])))
            r = re.search(r'r8=([0-9a-f]+) r9=([0-9a-f]+)', line)
            threads.setdefault(m.group(2), []).append(call + ((int(r.group(1), 16) & 0xFF, s32(int(r.group(2), 16))),) if r else call)
    return threads


def main():
    kinds = sys.argv[3].split(',')
    skip = int(sys.argv[4]) if len(sys.argv) > 4 else 0
    calls = ours(sys.argv[1], kinds)[skip:]
    mine = [c for c, _ in calls]
    best = (0, None, 0)
    for t, theirs in hits(sys.argv[2], kinds).items():
        for i in range(len(theirs)):
            if theirs[i:i + 3] == mine[:3]:
                n = 0
                while n < len(mine) and i + n < len(theirs) and theirs[i + n] == mine[n]:
                    n += 1
                best = max(best, (n, t, i))
    n, t, i = best
    if t is None:
        sys.exit(f'our first calls {mine[:3]} are in no thread')
    print(f'ours {len(mine)} calls; thread {t} matches the first {n}')
    theirs = hits(sys.argv[2], kinds)[t][i:]
    for k in range(max(0, n - 4), min(len(mine), n + CONTEXT)):
        b = theirs[k] if k < len(theirs) else None
        print(f'{k + skip:5} ours {mine[k]}  bds {b}{"" if b == mine[k] else "  <<<"}   [{calls[k][1]}]')


main()
