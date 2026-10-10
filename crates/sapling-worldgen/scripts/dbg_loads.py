"""What a BDS load did per chunk, from a debugger log of the three stage entries, and how two loads differ.

usage: py -3 -I dbg_loads.py <dbg.log> [<other dbg.log>] [--order <file>]

The log is a run of `bds_dbg_run.sh` with scripts/data/dbg_loads.json: `load` (OverworldGenerator::loadChunk),
`structfeat` (postProcessStructureFeatures) and `deco` (decorationPostProcessChunk), each logged with its chunk
and thread. Prints per stage the chunks, the ones run more than once, the threads, and the neighbour pairs that
break the parity phases; with a second log, the pairs of chunks whose 3x3 overlap (two apart or nearer) that
ran in the other order there. `--order` writes the first log's decoration order for `SAPLING_ORACLE_ORDER`;
with scripts/data/dbg_canon.json's log each line also holds where the thread generator stood at the chunk's
first MultifaceFeature draw (`x z draws`).
Why: docs/research/worldgen-bds-order.md.
"""
import re
import sys
from collections import Counter

PHASE = [0, 1, 3, 2]
STAGES = ("load", "structfeat", "deco")
HIT = re.compile(r"hit (\w+) t(\d+) .*?=\((-?\d+),(?: (-?\d+)\))?(?:\) \[[^\]]+\]=\((-?\d+),\))?")
DRAW = re.compile(r"hit draw t(\d+) .*rax=([0-9a-f]+)")


class Stream:
    """The outputs of BDS's `Random` seeded with 0 (MT19937), to place a logged draw in the stream."""

    def __init__(self):
        self.state, self.out = [0], []
        for i in range(1, 624):
            self.state.append((0x6C078965 * (self.state[-1] ^ (self.state[-1] >> 30)) + i) & 0xFFFFFFFF)

    def more(self):
        s = self.state
        for k in range(624):
            y = (s[k] & 0x80000000) | (s[(k + 1) % 624] & 0x7FFFFFFF)
            s[k] = s[(k + 397) % 624] ^ (y >> 1) ^ (0x9908B0DF if y & 1 else 0)
        for y in s:
            y ^= y >> 11
            y ^= (y << 7) & 0x9D2C5680
            y ^= (y << 15) & 0xEFC60000
            self.out.append(y ^ (y >> 18))

    def find(self, value, start):
        """The first position at or after `start` whose draw is `value`."""
        at = start
        while at < start + 4_000_000:
            if at >= len(self.out):
                self.more()
            elif self.out[at] == value:
                return at
            else:
                at += 1
        raise SystemExit(f"draw {value:x} is not within 4M draws of {start}: another seed, or a second worker?")


def phase(c):
    return PHASE[(c[0] & 1) | (c[1] & 1) << 1]


def read(path):
    """stage -> [(chunk, thread)] in log order."""
    runs = {stage: [] for stage in STAGES}
    entered, tries = {}, 0
    # dbg_canon.json: the value of each chunk's first MultifaceFeature draw places it in the generator's stream
    decorating, drawn, starts, stream = {}, 0, {}, Stream()
    for line in open(path):
        draw = DRAW.match(line)
        if draw:
            chunk = decorating.get(int(draw.group(1)))
            if chunk not in starts:
                drawn = starts[chunk] = stream.find(int(draw.group(2), 16) & 0xFFFFFFFF, drawn)
            continue
        m = HIT.match(line)
        if m and m.group(1) in runs:
            z = m.group(4) if m.group(4) is not None else m.group(5)
            # decoration is handed its 3x3 view: the logged corner is the chunk's minus one
            corner = m.group(1) == "deco"
            event = ((int(m.group(3)) + corner, int(z) + corner), int(m.group(2)))
            if corner:
                # an entry whose 3x3 lock fails returns to the queue: it counts once `decorate` follows
                entered[event[1]], tries = event, tries + 1
            else:
                runs[m.group(1)].append(event)
        elif line.startswith("hit decorate t"):
            thread = int(line.split()[2][1:])
            runs["deco"].append(entered.pop(thread))
            decorating[thread] = runs["deco"][-1][0]
    runs["tries"], runs["starts"] = tries, starts
    return runs


def first(events):
    place = {}
    for i, (chunk, _) in enumerate(events):
        place.setdefault(chunk, i)
    return place


def pairs(place, reach):
    for a in place:
        for dx in range(-reach, reach + 1):
            for dz in range(-reach, reach + 1):
                b = (a[0] + dx, a[1] + dz)
                if a < b and b in place:
                    yield a, b


def describe(name, runs):
    for stage in STAGES:
        events = runs[stage]
        place = first(events)
        again = {c: n for c, n in Counter(c for c, _ in events).items() if n > 1}
        threads = Counter(t for _, t in events)
        against = sum((place[a] < place[b]) != (phase(a) < phase(b)) for a, b in pairs(place, 1))
        total = sum(1 for _ in pairs(place, 1))
        print(f"{name} {stage}: {len(place)} chunks, {len(again)} more than once {sorted(again.items())[:6]}, "
              f"threads {sorted(threads.values(), reverse=True)}, neighbour pairs against parity {against} of {total}")
    print(f"{name} deco: {runs['tries']} entries for {len(runs['deco'])} decorations (the rest failed the 3x3 lock)")


def compare(a, b):
    for stage in STAGES:
        pa, pb = first(a[stage]), first(b[stage])
        common = {c: pa[c] for c in pa if c in pb}
        for reach, label in ((1, "neighbours"), (2, "3x3 overlaps")):
            flipped = [(x, y) for x, y in pairs(common, reach) if (pa[x] < pa[y]) != (pb[x] < pb[y])]
            total = sum(1 for _ in pairs(common, reach))
            print(f"{stage}: {len(flipped)} of {total} {label} in the other order {flipped[:4]}")
        same = [c for c, _ in a[stage]] == [c for c, _ in b[stage]]
        print(f"{stage}: {len(pa)} and {len(pb)} chunks, {len(common)} in both, same sequence: {same}")


def grid(events):
    """Each chunk's place in a stage's order, as a map (x across, z down)."""
    place = first(events)
    xs, zs = [c[0] for c in place], [c[1] for c in place]
    print("      " + "".join(f"{x:5d}" for x in range(min(xs), max(xs) + 1)))
    for z in range(min(zs), max(zs) + 1):
        print(f"{z:6d}" + "".join(f"{place.get((x, z), -1):5d}" for x in range(min(xs), max(xs) + 1)))


def main():
    args = sys.argv[1:]
    order = args.pop(args.index("--order") + 1) if "--order" in args else None
    show = args.pop(args.index("--grid") + 1) if "--grid" in args else None
    logs = [a for a in args if not a.startswith("--")]
    runs = [read(path) for path in logs]
    if show:
        grid(runs[0][show])
    for path, run in zip(logs, runs):
        describe(path.replace("\\", "/").split("/")[-3] if path.count("/") + path.count("\\") > 1 else path, run)
    if len(runs) > 1:
        compare(runs[0], runs[1])
    if order:
        place = first(runs[0]["deco"])
        starts = runs[0]["starts"]
        with open(order, "w", newline="\n") as out:
            out.writelines(f"{c[0]} {c[1]}" + (f" {starts[c]}\n" if c in starts else "\n") for c in sorted(place, key=place.get))


main()
