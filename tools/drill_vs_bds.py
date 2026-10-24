"""Compares a sapling drill capture with BDS's, line of the drill by line: command answers and chat exactly,
every other packet by how many of each kind each bot got. Positions and entity ids differ between the two
worlds, so they are not compared here (tools/drill_motion.py compares two sapling runs packet by packet).
usage: py -3 -I tools/drill_vs_bds.py <bds capture> <sapling capture> [--all]   (--all: every differing line)
       py -3 -I tools/drill_vs_bds.py --sweep <bds dir> <sapling dir> <word>...   one summary line per word
"""
import re
import sys
from collections import Counter
from pathlib import Path

EVENT = re.compile(r"^\s*\d+\s+(actor|watcher)\s+(\w+)\s?(.*)$")
ANSWER = re.compile(r"success_count=(\d+) data=\S+ (.*?) success=(\w+) (.*)$")
MESSAGE = re.compile(r'message: "(.*?)" \}\)')
# Positions in answers (/tp's "6.43, -19.00") differ with where each world put the bots.
DECIMAL = re.compile(r"-?\d+\.\d\d\b")
# Periodic or per-connection packets that say nothing about the drill.
IGNORED = {"SyncWorldClocks", "SetTime", "NetworkStackLatency", "LevelChunk", "NetworkChunkPublisherUpdate"}


def sections(path):
    """[(drill line, [(bot, kind, detail)])] in capture order."""
    out = []
    for line in Path(path).read_text(encoding="utf-8", errors="replace").splitlines():
        if line.startswith("## drill starts at "):
            continue
        if line.startswith("## "):
            out.append((line[3:], []))
            continue
        m = EVENT.match(line)
        if not m or not out:
            continue
        bot, kind, rest = m.groups()
        if kind in IGNORED:
            continue
        detail = ""
        if kind == "CommandOutput" and (a := ANSWER.search(rest)):
            detail = DECIMAL.sub("#.##", f"{a.group(2)} success={a.group(3)} {a.group(4)}")
        elif kind == "Text" and (t := MESSAGE.search(rest)):
            detail = DECIMAL.sub("#.##", t.group(1))
        out[-1][1].append((bot, kind, detail))
    return out


def differences(bds, sap):
    """[(drill line, [difference])] for the lines whose packets differ."""
    found = []
    for (line, b), (_, s) in zip(bds, sap):
        notes = []
        words_b = [(bot, d) for bot, k, d in b if k in ("CommandOutput", "Text")]
        words_s = [(bot, d) for bot, k, d in s if k in ("CommandOutput", "Text")]
        if words_b != words_s:
            notes += [f"BDS said   {bot}: {d}" for bot, d in words_b if (bot, d) not in words_s]
            notes += [f"sapling said {bot}: {d}" for bot, d in words_s if (bot, d) not in words_b]
        kb = Counter((bot, k) for bot, k, _ in b if k not in ("CommandOutput", "Text"))
        ks = Counter((bot, k) for bot, k, _ in s if k not in ("CommandOutput", "Text"))
        notes += [f"{bot} {k}: BDS {kb[(bot, k)]}, sapling {ks[(bot, k)]}" for bot, k in sorted(set(kb) | set(ks)) if kb[(bot, k)] != ks[(bot, k)]]
        if notes:
            found.append((line, notes))
    if len(bds) != len(sap):
        found.append(("(drill)", [f"{len(bds)} lines on BDS, {len(sap)} on sapling"]))
    return found


def sweep(bds_dir, sap_dir, words):
    for word in words:
        bds, sap = Path(bds_dir) / f"bds-{word}.txt", Path(sap_dir) / f"{word}.txt"
        if not bds.exists() or not sap.exists():
            print(f"{word:16} missing {'BDS' if not bds.exists() else 'sapling'} capture")
            continue
        sb, ss = sections(bds), sections(sap)
        diff = differences(sb, ss)
        answers = sum(1 for _, notes in diff for n in notes if " said " in n)
        print(f"{word:16} {len(sb) - len(diff):3}/{len(sb):<3} lines alike, {answers:3} answer/chat differences")


def main():
    if sys.argv[1] == "--sweep":
        sweep(sys.argv[2], sys.argv[3], sys.argv[4:])
        return
    diff = differences(sections(sys.argv[1]), sections(sys.argv[2]))
    for line, notes in diff if "--all" in sys.argv else diff[:20]:
        print(f"## {line}")
        for note in notes[:12]:
            print(f"   {note[:300]}")


main()
