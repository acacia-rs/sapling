"""Compares the watcher's per-entity motion packets of two drill captures, as tools/drill-ab.sh writes them.
usage: py -3 -I tools/drill_motion.py <dir> <side a>,<side b> <word>...   reads <dir>/<side>/<word>.txt
"""
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

LINE = re.compile(r"^\s*(\d+)\s+watcher\s+(\w+)\s+(.*)$")
RID = re.compile(r"runtime_entity_id: (\d+)")
NUM = re.compile(r"-?\d+\.\d+")
MOTION = ("Move", "Motion", "AddItemEntity", "AddActor", "AddEntity")
CLOCKS = {"SyncWorldClocks", "SetTime"}


def motion(path):
    kinds = Counter()
    per = defaultdict(list)
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        m = LINE.match(line)
        if not m:
            continue
        kind, body = m.group(2), m.group(3)
        kinds[kind] += 1
        if not any(k in kind for k in MOTION):
            continue
        rid = RID.search(body)
        per[(kind, rid.group(1) if rid else "?")].append([float(x) for x in NUM.findall(body)])
    return kinds, per


def compare(word, sides):
    (kinds_a, a), (kinds_b, b) = sides
    keys = sorted(set(a) | set(b))
    same = differ = 0
    worst = 0.0
    notes = []
    for k in keys:
        xs, ys = a.get(k, []), b.get(k, [])
        if len(xs) != len(ys):
            notes.append(f"  {k}: {len(xs)} vs {len(ys)} packets")
        for x, y in zip(xs, ys):
            gap = max((abs(p - q) for p, q in zip(x, y)), default=0.0) if len(x) == len(y) else float("inf")
            if gap == 0.0:
                same += 1
            else:
                differ += 1
                worst = max(worst, gap)
    # Block updates, sounds and events aren't compared one by one, but their counts are; clock syncs
    # are periodic and follow the run's length, not the drill.
    notes += [f"  {kind}: {kinds_a[kind]} vs {kinds_b[kind]} packets" for kind in sorted(set(kinds_a) | set(kinds_b))
              if kinds_a[kind] != kinds_b[kind] and not any(k in kind for k in MOTION) and kind not in CLOCKS]
    print(f"{word}: {same} motion packets identical, {differ} differ (largest number gap {worst:.6g}); kinds {sorted({k[0] for k in keys})}")
    for note in notes[:8]:
        print(note)
    return differ == 0 and not notes


def main():
    root, sides, words = Path(sys.argv[1]), sys.argv[2].split(","), sys.argv[3:]
    ok = all([compare(w, [motion(root / side / f"{w}.txt") for side in sides]) for w in words])
    sys.exit(0 if ok else 1)


main()
