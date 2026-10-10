"""Per-world totals of the oracle's `floor:` lines (tests/oracle_support/floor.rs).

usage: py -3 -I oracle_floor.py <oracle log> [--regions] [<earlier log>]

Columns: wrong + missing cells; cells holding a lichen or sculk block on either side; of those, the ones in
our decoration's thread-local reach, the ones within 15 blocks of a lichen or sculk cell BDS's two runs
disagree on, and the floor (in reach and within 4 of such a cell); other cells (liquids) that are floor the
same way; dripstone, sulfur and lush-cave cells within 10 blocks of a twin disagreement (their scans stop
there); the deterministic residue (all but floor, other floor and those scans); and `no-tl`, the rest once
every lichen and sculk cell is set aside (what logs before 2026-10-10 called deterministic).
With an earlier log, the wrong + missing column shows `before -> after`; an old-format log gives its totals.
"""

import collections
import re
import sys

LINE = re.compile(r'floor: world=(\S+) region="([^"]+)" wrong\+missing=(\d+) thread_local_blocks=(\d+) \(reached (\d+), twin within \d+ (\d+), '
                  r'floor (\d+)\) other_floor=(\d+) scans=(\d+) \(twin-confirmed (\d+)\) deterministic=(\d+)')
OLD = re.compile(r'floor: world=(\S+) region="([^"]+)" wrong\+missing=(\d+) thread_local_blocks=(\d+) \(twin-confirmed (\d+)\) '
                 r'scans=(\d+) \(twin-confirmed (\d+)\) deterministic=(\d+)')
# The 20261003 world holds two tests' regions.
SPLIT = {'chunks -16..15': 'near origin', 'chunks 100000..100007': 'far out'}


def read(path, by_region):
    """key -> [total, local, reached, twin15, floor, other, scans, scan_near, deterministic]; old logs fill what they have."""
    rows = collections.OrderedDict()
    text = open(path, encoding='utf-8', errors='replace').read()
    new = list(LINE.finditer(text))
    for match in new or OLD.finditer(text):
        world, region, *numbers = match.groups()
        numbers = list(map(int, numbers))
        if not new:
            total, local, near, scans, scan_near, rest = numbers
            numbers = [total, local, 0, near, 0, 0, scans, scan_near, rest]
        still = ' (still)' if region.endswith(' (still)') else ''
        key = f'{world} {region}' if by_region else SPLIT.get(region.removesuffix(still), world.removesuffix('-still')) + still
        row = rows.setdefault(key, [0] * 9)
        for i, n in enumerate(numbers):
            row[i] += n
    return rows


def main():
    args = [a for a in sys.argv[1:] if not a.startswith('--')]
    by_region = '--regions' in sys.argv
    rows = read(args[0], by_region)
    before = read(args[1], by_region) if len(args) > 1 else {}
    print(f'{"world":<44} {"wrong+missing":>16} {"lichen/sculk":>13} {"reached":>8} {"twin15":>7} {"floor":>6} {"other":>6} {"scans (twin)":>13} {"deterministic":>14} {"no-tl":>7}')
    for key, (total, local, reached, near, floor, other, _, scan_near, rest) in sorted(rows.items()):
        was = f'{before[key][0]} -> ' if key in before else ''
        if by_region and key in before and before[key][0] == total:
            continue
        print(f'{key:<44} {was + str(total):>16} {local:>13} {reached:>8} {near:>7} {floor:>6} {other:>6} {scan_near:>13} {rest:>14} {total - local - scan_near:>7}')


main()
