"""Per-world totals of the oracle's `floor:` lines (tests/oracle_support/floor.rs).

usage: py -3 -I oracle_floor.py <oracle log> [--regions] [<earlier log>]

Columns: wrong + missing cells; cells holding a lichen or sculk block on either side (and those within
15 blocks of a lichen or sculk cell BDS's two runs disagree on); dripstone, sulfur and lush-cave cells
within 10 blocks of such a cell (their scans stop there); the deterministic residue (all but those two;
the upper bound keeps the lichen and sculk cells the twin does not confirm). With an earlier log, the
wrong + missing column shows `before -> after`.
"""

import collections
import re
import sys

LINE = re.compile(r'floor: world=(\S+) region="([^"]+)" wrong\+missing=(\d+) thread_local_blocks=(\d+) \(twin-confirmed (\d+)\) '
                  r'scans=(\d+) \(twin-confirmed (\d+)\) deterministic=(\d+) \(upper bound (\d+)\)')
# The 20261003 world holds two tests' regions.
SPLIT = {'chunks -16..15': 'near origin', 'chunks 100000..100007': 'far out'}


def read(path, by_region):
    rows = collections.OrderedDict()
    with open(path, encoding='utf-8', errors='replace') as f:
        for match in LINE.finditer(f.read()):
            world, region, *numbers = match.groups()
            still = ' (still)' if region.endswith(' (still)') else ''
            key = f'{world} {region}' if by_region else SPLIT.get(region.removesuffix(still), world) + still
            row = rows.setdefault(key, [0] * 7)
            for i, n in enumerate(numbers):
                row[i] += int(n)
    return rows


def main():
    args = [a for a in sys.argv[1:] if not a.startswith('--')]
    by_region = '--regions' in sys.argv
    rows = read(args[0], by_region)
    before = read(args[1], by_region) if len(args) > 1 else {}
    print(f'{"world":<44} {"wrong+missing":>16} {"lichen/sculk":>13} {"(twin)":>7} {"scans (twin)":>13} {"deterministic":>14} {"(upper)":>8}')
    for key, (total, local, local_near, _, scan_near, rest, upper) in sorted(rows.items()):
        was = f'{before[key][0]} -> ' if key in before else ''
        if by_region and key in before and before[key][0] == total:
            continue
        print(f'{key:<44} {was + str(total):>16} {local:>13} {local_near:>7} {scan_near:>13} {rest:>14} {upper:>8}')


main()
