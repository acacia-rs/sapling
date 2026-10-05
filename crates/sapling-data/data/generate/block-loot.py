"""Java 26.3 block loot tables -> block-loot.tsv + block-loot-nothing.txt. See loot-and-tags.md.

usage: py -3 block-loot.py
"""
import gzip
import json
import os
import sys
import zipfile
from collections import Counter, defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from bedrock_names import load_bedrock_names, to_bedrock  # noqa: E402
from loot_eval import Table  # noqa: E402

JAVA = "C:/Users/Hexay/acacia/research/java-truth/"
LOOT = "data/minecraft/loot_table/blocks/"
SIMPLE_DROP = (1, 1, (1.0,), "none", ())


def fmt(x):
    if isinstance(x, float) and x == int(x):
        return str(int(x))
    return repr(round(x, 9)) if isinstance(x, float) else str(x)


def load_state_domains():
    """block -> {property: set(values)} from the state keys of java-truth/quads.json.gz (full registry)."""
    with gzip.open(JAVA + "quads.json.gz", "rt", encoding="utf-8") as f:
        keys = json.load(f)["states"].keys()
    domains = {}
    for key in keys:
        block, _, props = key[len("minecraft:"):].partition("[")
        d = domains.setdefault(block, defaultdict(set))
        for kv in props.rstrip("]").split(","):
            if kv:
                k, v = kv.split("=")
                d[k].add(v)
    return domains


def nothing_reason(dropping):
    if not dropping:
        return "empty_loot_table"
    if "FF" in dropping:
        return None
    if "FT" in dropping:
        return "shears_or_silk_touch_only" if "TF" in dropping else "shears_only"
    return "silk_touch_only"


def main():
    bedrock = load_bedrock_names()
    domains = load_state_domains()
    z = zipfile.ZipFile(JAVA + "client-26.3.jar")
    tables = sorted(n[len(LOOT):-5] for n in z.namelist() if n.startswith(LOOT) and n.endswith(".json"))
    out_rows, nothing, warnings, simple = [], [], defaultdict(list), 0
    # Blocks.wallVariant(base, ..) gives a wall block its base block's loot table: no file of its own.
    aliases = {b: b.replace("wall_", "", 1) for b in domains if b not in tables and "wall_" in b}
    assert all(base in tables for base in aliases.values()), aliases
    for block in sorted(tables + list(aliases)):
        table = aliases.get(block, block)
        t = Table(table, json.loads(z.read(LOOT + table + ".json")), domains.get(table, {}))
        rows, dropping = t.rows()
        for w in t.warnings:
            warnings[w].append(block)
        reason = nothing_reason(dropping)
        if reason:
            nothing.append((block, reason))
        if len(rows) == 1 and rows[0] == ("default", block) + SIMPLE_DROP:
            simple += 1
            continue
        for label, item, lo, hi, chance, fortune, notes in rows:
            notes = list(notes)
            if "location_check ignored" in t.warnings:
                notes.append("either half, only while the other half is still in place (location_check): once per plant")
            if block in aliases:
                notes.append("loot_table_of=" + table)
            if len(set(chance)) > 1:
                notes.append("chance_by_fortune=" + ",".join(fmt(c) for c in chance))
            b_item = "" if item.startswith("dynamic:") else to_bedrock(item, bedrock)
            b_item = "minecraft:" + b_item if b_item else ""
            out_rows.append((block, label, item, fmt(lo), fmt(hi), fmt(chance[0]), fortune, "; ".join(notes), b_item, "yes" if b_item else "no"))
    nothing += [(b, "no_loot_table") for b in sorted(domains) if b not in tables and b not in aliases]
    missing_state = [b for b in tables if b not in domains]
    out_rows.sort(key=lambda r: (r[0], r[1], r[2]))
    with open(os.path.join(HERE, "block-loot.tsv"), "w", encoding="utf-8", newline="\n") as f:
        f.write("block\tcondition\titem\tmin\tmax\tchance\tfortune\tnotes\tbedrock_item\tbedrock_item_known\n")
        for r in out_rows:
            f.write("\t".join(r) + "\n")
    with open(os.path.join(HERE, "block-loot-nothing.txt"), "w", encoding="utf-8", newline="\n") as f:
        f.write("block\treason\n")
        for b, why in sorted(nothing, key=lambda x: (x[1], x[0])):
            f.write(f"{b}\t{why}\n")

    print(f"tables={len(tables)} registry_blocks={len(domains)} simple_self_drop={simple} rows={len(out_rows)} "
          f"blocks_in_tsv={len(set(r[0] for r in out_rows))} tables_without_states={missing_state}")
    print("nothing:", dict(Counter(w for _b, w in nothing)))
    print("conditions:", Counter(r[1] for r in out_rows).most_common(70))
    print("fortune:", Counter(r[6].split(":")[0] for r in out_rows).most_common())
    print("no bedrock name:", sorted(set(r[2] for r in out_rows if not r[8])))
    print("renamed:", sorted(set((r[2], r[8]) for r in out_rows if r[8] and r[8] != "minecraft:" + r[2])))
    if "--items" in sys.argv:
        print("same-name items:", " ".join(sorted(set(r[2] for r in out_rows if r[8] == "minecraft:" + r[2]))))
    for pat in sys.argv[1:]:
        if pat.startswith("show="):
            for r in out_rows:
                if r[0] in pat[5:].split(","):
                    print("  ", " | ".join(r))
    for w, bl in sorted(warnings.items()):
        print("WARN", w, len(bl), bl[:6])


if __name__ == "__main__":
    main()
