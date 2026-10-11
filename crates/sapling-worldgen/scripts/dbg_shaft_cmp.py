"""Mineshaft pieces a BDS run post-processed against the replay's, piece by piece.

usage: py -3 -I dbg_shaft_cmp.py <dbg.log> <structure_replay_draws output> [--all]

The log is a `bds_dbg_run.sh` run with scripts/data/dbg_shaft.json: every room, corridor, crossing and stairs
`postProcess` with the piece's box, the chunk's box and the chunk generator's position (the index into its 624
words: the `Random` handed to pieces holds its generator at +8, so +0x9d4). The other file is `examples/structure_replay_draws` under that run's
`SAPLING_ORACLE_STRUCTURE_ORDER`. Prints the chunks whose piece sequences or draw positions differ, each with
its first differing piece, in the run's order; `--all` prints every piece of those chunks.
Why: docs/research/worldgen-bds-order.md, "Replaying a run".
"""
import re
import sys

PIECE = re.compile(r"hit (room|corridor|crossing|stairs) t\d+ .*?\]=\(([^)]*)\) \[[^\]]+\]=\(([^)]*)\) \[[^\]]+\]=\((?:\d+, \d+, )?(\d+),?\)")


def position(draws):
    """`Random`'s index after `draws` raw draws from a fresh seed."""
    return 624 if draws == 0 else (draws - 1) % 624 + 1


def theirs(path):
    """chunk -> [(piece box, generator index, kind)], chunks in log order."""
    chunks = {}
    for line in open(path):
        m = PIECE.match(line)
        if m:
            box = tuple(int(v) for v in m.group(2).split(","))
            area = [int(v) for v in m.group(3).split(",")]
            chunks.setdefault((area[0] >> 4, area[2] >> 4), []).append((box, int(m.group(4)), m.group(1)))
    return chunks


def ours(path):
    chunks = {}
    for line in open(path):
        parts = line.split(" ", 9)
        if len(parts) < 10 or line.startswith(" "):
            continue
        try:
            n = [int(v) for v in parts[:9]]
        except ValueError:
            continue
        chunks.setdefault((n[0], n[1]), []).append((tuple(n[2:8]), n[8], parts[9].strip()))
    return chunks


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    bds, port = theirs(args[0]), ours(args[1])
    same = 0
    for chunk in list(bds) + [c for c in port if c not in bds]:
        a, b = bds.get(chunk, []), port.get(chunk, [])
        rows = [(x[0], x[1], x[2]) for x in a]
        mine = [(y[0], position(y[1]), y[2]) for y in b]
        first = next((i for i in range(max(len(rows), len(mine))) if i >= len(rows) or i >= len(mine) or rows[i][:2] != mine[i][:2]), None)
        if first is None:
            same += 1
            continue
        print(f"chunk {chunk[0]},{chunk[1]}: {len(rows)} pieces in BDS, {len(mine)} in ours, first difference at {first}")
        span = range(max(len(rows), len(mine))) if "--all" in sys.argv else range(max(first - 1, 0), min(first + 3, max(len(rows), len(mine))))
        for i in span:
            left = f"{rows[i][2]} {rows[i][0]} at {rows[i][1]}" if i < len(rows) else "-"
            right = f"{mine[i][0]} at {mine[i][1]} ({b[i][1]} draws) {mine[i][2]}" if i < len(mine) else "-"
            print(f"  {i}: BDS {left} | ours {right}")
    print(f"{same} chunks agree piece for piece")


main()
