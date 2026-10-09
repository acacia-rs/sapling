"""Clusters of the oracle's terrain-stage errors from `SAPLING_ORACLE_DUMP`'s `.terrain` files (tests/oracle_support/dump.rs).

usage: py -3 -I terrain_residue.py <dump dir> [world prefix] [--cells] [--reach N]

Per world and region: the stable errors of the stages before the features (shape, surface, carvers, veins,
deepslate, floor, lakes, structures) per stage, then their clusters (cells within N blocks of one another on
every axis, default 2), largest first: size, bounding box, stages, the (ours, BDS) pairs and the flags of our
stages that touched the cells (c carved, s structured, l laked, f featured, r reshaped). `--cells` lists the cells.
"""

import collections
import os
import sys


def rows(path):
    with open(path) as f:
        for line in f:
            x, y, z, stage, ours, bds, flags = line.rstrip('\n').split('\t')
            yield (int(x), int(y), int(z)), stage, ours, bds, flags


def clusters(cells, reach):
    """Union of cells whose coordinates differ by at most `reach` on every axis."""
    buckets = collections.defaultdict(list)
    for p in cells:
        buckets[tuple(v // (reach + 1) for v in p)].append(p)
    seen, out = set(), []
    for start in cells:
        if start in seen:
            continue
        seen.add(start)
        group, stack = [], [start]
        while stack:
            p = stack.pop()
            group.append(p)
            b = tuple(v // (reach + 1) for v in p)
            for dx in (-1, 0, 1):
                for dy in (-1, 0, 1):
                    for dz in (-1, 0, 1):
                        for q in buckets.get((b[0] + dx, b[1] + dy, b[2] + dz), ()):
                            if q not in seen and all(abs(u - v) <= reach for u, v in zip(p, q)):
                                seen.add(q)
                                stack.append(q)
        out.append(group)
    return sorted(out, key=len, reverse=True)


def main():
    argv, reach = sys.argv[1:], 2
    if '--reach' in argv:
        at = argv.index('--reach')
        reach = int(argv[at + 1])
        del argv[at:at + 2]
    args = [a for a in argv if not a.startswith('--')]
    root, prefix = args[0], args[1] if len(args) > 1 else ''
    total = collections.Counter()
    for file in sorted(os.listdir(root)):
        if not file.endswith('.terrain') or not file.startswith(prefix):
            continue
        cells = {p: (stage, ours, bds, flags) for p, stage, ours, bds, flags in rows(os.path.join(root, file))}
        if not cells:
            continue
        stages = collections.Counter(v[0] for v in cells.values())
        total.update(stages)
        print(f'== {file[:-8]}: {len(cells)}: ' + ', '.join(f'{s} {n}' for s, n in stages.most_common()))
        for group in clusters(list(cells), reach):
            lo = [min(p[i] for p in group) for i in range(3)]
            hi = [max(p[i] for p in group) for i in range(3)]
            pairs = collections.Counter(f'{cells[p][0]} {cells[p][1]}|{cells[p][2]} [{cells[p][3]}]' for p in group)
            print(f'  {len(group):>5} at {lo[0]},{lo[1]},{lo[2]}..{hi[0]},{hi[1]},{hi[2]} chunk {lo[0] >> 4},{lo[2] >> 4}: '
                  + ', '.join(f'{k} {n}' for k, n in pairs.most_common(5)))
            if '--cells' in sys.argv:
                for p in sorted(group):
                    print('       ', *p, *cells[p])
    print('all: ' + ', '.join(f'{s} {n}' for s, n in total.most_common()))


main()
