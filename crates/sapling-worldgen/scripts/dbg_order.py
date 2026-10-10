"""The order in which BDS decorated chunks in a debugger run.

usage: py -3 -I dbg_order.py <dbg.log> <x0> <z0> <x1> <z1> [<order file>]

A chunk's place is its first logged hit whose position lies in it, so log a feature that rules place at their own
positions (`dripstone_cluster_feature` 149d61dc0, a tree) with a filter box around the whole region. Prints the
neighbour pairs that break the parity phases, each chunk's place relative to the region (in, ring, out), then the
order as `x,z:<phase><i|r|o>`. The order file (`x z` per line) is what the oracle test's `SAPLING_ORACLE_ORDER`
reads to decorate as that run did.
"""
import re
import sys

PHASE = [0, 1, 3, 2]


def phase(c):
    return PHASE[(c[0] & 1) | (c[1] & 1) << 1]


def main():
    x0, z0, x1, z1 = map(int, sys.argv[2:6])
    first, n = {}, 0
    for line in open(sys.argv[1]):
        m = re.match(r'hit \w+ t\d+ pos=\((-?\d+), (-?\d+), (-?\d+)\)', line)
        if m:
            n += 1
            first.setdefault((int(m.group(1)) >> 4, int(m.group(3)) >> 4), n)

    def where(c):
        return ['in', 'ring', 'out'][min(max(x0 - c[0], c[0] - x1, z0 - c[1], c[1] - z1, 0), 2)]

    print(f'{len(first)} chunks decorated, {n} hits')
    good = bad = 0
    for a in sorted(first):
        for d in ((1, 0), (0, 1), (1, 1), (1, -1)):
            b = (a[0] + d[0], a[1] + d[1])
            if b not in first:
                continue
            lo, hi = sorted((a, b), key=phase)
            if first[lo] < first[hi]:
                good += 1
            else:
                bad += 1
                print(f'  {hi} phase {phase(hi)} ({where(hi)}) before {lo} phase {phase(lo)} ({where(lo)})')
    print(f'neighbour pairs in parity order {good}, against {bad}')
    print('order:', ' '.join(f'{c[0]},{c[1]}:{phase(c)}{where(c)[0]}' for c in sorted(first, key=first.get)))
    if len(sys.argv) > 6:
        with open(sys.argv[6], 'w', newline='\n') as out:
            out.writelines(f'{c[0]} {c[1]}\n' for c in sorted(first, key=first.get))


main()
