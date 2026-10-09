"""Breakdown of the oracle's liquid residue (stage L rows of `SAPLING_ORACLE_DUMP` files, tests/oracle_support/dump.rs).

usage: py -3 liquid_residue.py <dump dir> [world prefix] [--by-region] [--cells <kind>] [--compare <other dump dir>]

Per world (or region): stable liquid errors by kind (missing: BDS has fluid, ours air; extra: ours fluid, BDS
air; depth: both fluid, other depth or liquid) and cause, the first that applies to any cell of its flow (26-connected
liquid errors):
  tl      within 4 blocks of a glow lichen or sculk difference (either side or the twin: thread-local in BDS)
  feat    within 2 blocks of another stable error (a feature block, or fluid against solid, e.g. a spring's rock
          cell: stage W): a block that started or steered the flow differs
  adj     within 1 block of a BDS structure block of the cell's chunk
  struct  within 6 blocks of one
  rest    none of these
`--compare` also splits each by whether the other dump (e.g. a run with SAPLING_LIQUID_GAME_TICKS set) still has
the cell wrong: cells it fixes are game time, not decoration.
"""

import collections
import os
import sys

THREAD_LOCAL = ('glow_lichen', 'sculk')


def rows(path):
    with open(path) as f:
        head = f.readline().rstrip('\n').split('\t')
        for line in f:
            values = line.rstrip('\n').split('\t')
            yield dict(zip(head, values + [''] * (len(head) - len(values))))


def fluid(cell):
    name = cell.split('[', 1)[0]
    return 'lava' if 'lava' in name else 'water' if 'water' in name else None


def kind(row):
    ours, bds = fluid(row['ours']), fluid(row['bds'])
    if ours and bds:
        return f'depth {bds}'
    return f'missing {bds}' if bds else f'extra {ours}'


def key(row):
    return int(row['x']), int(row['y']), int(row['z'])


class Near:
    def __init__(self, points):
        self.cells = collections.defaultdict(list)
        for p in points:
            self.cells[(p[0] >> 3, p[1] >> 3, p[2] >> 3)].append(p)

    def within(self, p, reach):
        b = (p[0] >> 3, p[1] >> 3, p[2] >> 3)
        return any(max(abs(u - v) for u, v in zip(p, q)) <= reach
                   for dx in (-1, 0, 1) for dy in (-1, 0, 1) for dz in (-1, 0, 1)
                   for q in self.cells.get((b[0] + dx, b[1] + dy, b[2] + dz), ()))


def cell_cause(row, local, features):
    p = key(row)
    d = row.get('sdist', '-')
    d = 99 if d in ('', '-') else int(d)
    if local.within(p, 4):
        return 0
    if features.within(p, 2):
        return 1
    return 2 if d <= 1 else 3 if d <= 6 else 4


CAUSES = ['tl', 'feat', 'adj', 'struct', 'rest']


def causes_by_group(liquids, local, features):
    """Each flow (cells 26-connected) takes the strongest cause of any of its cells."""
    at = {key(r): r for r in liquids}
    best, seen = {}, set()
    for start in at:
        if start in seen:
            continue
        group, stack = [], [start]
        seen.add(start)
        while stack:
            p = stack.pop()
            group.append(p)
            for d in [(a, b, c) for a in (-1, 0, 1) for b in (-1, 0, 1) for c in (-1, 0, 1)]:
                q = (p[0] + d[0], p[1] + d[1], p[2] + d[2])
                if q in at and q not in seen:
                    seen.add(q)
                    stack.append(q)
        c = min(cell_cause(at[p], local, features) for p in group)
        for p in group:
            best[p] = CAUSES[c]
    return best


def load(path):
    liquids, local, features = [], [], []
    for row in rows(path):
        p = key(row)
        if any(t in row[k] for k in ('ours', 'bds', 'twin') for t in THREAD_LOCAL):
            local.append(p)
        if row.get('stage') == 'L':
            liquids.append(row)
        elif row['stable'] == 'S':
            features.append(p)
    local, features = Near(local), Near(features)
    return liquids, causes_by_group(liquids, local, features)


def main():
    args = sys.argv[1:]
    root = args[0]
    prefix = args[1] if len(args) > 1 and not args[1].startswith('--') else ''
    want = args[args.index('--cells') + 1] if '--cells' in args else None
    other = args[args.index('--compare') + 1] if '--compare' in args else None
    by_region = '--by-region' in args
    groups = collections.defaultdict(list)
    for file in sorted(os.listdir(root)):
        if file.endswith('.tsv') and file.startswith(prefix):
            world, region = file[:-4].split('.', 1)
            groups[f'{world} {region}' if by_region or world == '20261003' else world].append(file)
    for name, files in groups.items():
        table, causes = collections.Counter(), collections.Counter()
        for file in files:
            still = {key(r) for r in rows(os.path.join(other, file)) if r.get('stage') == 'L'} if other else None
            liquids, cause = load(os.path.join(root, file))
            for row in liquids:
                k, c = kind(row), cause[key(row)]
                tag = c + ('' if still is None else ' kept' if key(row) in still else ' fixed')
                table[(k, tag)] += 1
                causes[tag] += 1
                if want and want in k:
                    print(file, row['x'], row['y'], row['z'], row['before'], row['ours'], row['bds'], tag, sep='\t')
        print(f'== {name}: {sum(causes.values())}  ' + ' '.join(f'{t}:{n}' for t, n in sorted(causes.items())))
        kinds = collections.Counter()
        for (k, _), n in table.items():
            kinds[k] += n
        for k, n in kinds.most_common():
            parts = ' '.join(f'{t}:{c}' for (kk, t), c in sorted(table.items()) if kk == k)
            print(f'  {k:>14} {n:>6}  {parts}')


main()
