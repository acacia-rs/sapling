"""Symbolic evaluator for Java block loot tables (26.3 JSON shape). Used by block-loot.py; see loot-and-tags.md.

A table is evaluated under every combination of the deterministic inputs it reads (silk touch, shears,
other tool predicates, block-state values); assignments that give the same drop are merged into one row.
"""
import itertools
import json
from collections import Counter, defaultdict

OTHER = "*"
ZERO = (0.0,)
PREDICATE_REFS = {"minecraft:tool/can_shear": "shears", "minecraft:tool/can_silk_touch": "silk"}
TOOL_LABELS = {
    frozenset("TT TF FT FF".split()): "",
    frozenset("TT TF".split()): "silk_touch",
    frozenset("TT FT".split()): "shears",
    frozenset("TT TF FT".split()): "shears_or_silk_touch",
    frozenset("FT FF".split()): "no_silk_touch",
    frozenset("TF FF".split()): "no_shears",
    frozenset(["FF"]): "no_shears_no_silk_touch",
    frozenset(["FT"]): "shears_no_silk_touch",
    frozenset(["TF"]): "silk_touch_no_shears",
    frozenset(["TT"]): "shears_and_silk_touch",
}


class Unknown(Exception):
    """A state property deliberately left out of the enumeration (see Table.counter_props)."""


def strip(s):
    return s[len("minecraft:"):] if s.startswith("minecraft:") else s


def as_list(v):
    if v is None:
        return []
    return v if isinstance(v, list) else [v]


def compact(o):
    return json.dumps(o, separators=(",", ":"), sort_keys=True)


def mul(a, b):
    if a is None or b is None:
        return a if b is None else b
    n = max(len(a), len(b))
    return tuple(a[min(i, len(a) - 1)] * b[min(i, len(b) - 1)] for i in range(n))


def number(c):
    if isinstance(c, (int, float)):
        return c, c, None
    t = strip(c["type"])
    if t == "constant":
        return c["value"], c["value"], None
    if t == "uniform":
        return c["min"], c["max"], None
    if t == "binomial":
        return 0, c["n"], f"count~binomial(n={c['n']},p={c['p']})"
    return 0, 0, "count:" + compact(c)


def sort_key(v):
    return (0, int(v), "") if v.lstrip("-").isdigit() else (1, 0, v)


def expand_tool(values):
    return frozenset(s + h for c in values for s in ("TF" if c[0] == "x" else c[0]) for h in ("TF" if c[1] == "x" else c[1]))


class Table:
    def __init__(self, block, data, state_domains):
        self.block = block
        self.data = data
        self.domains = state_domains
        self.warnings = set()
        self.bools = set()
        self.props = defaultdict(set)
        self.counter_props = set()

    def cond(self, c):
        if isinstance(c, str):
            name = PREDICATE_REFS.get(c)
            if name is None:
                self.warnings.add(f"unknown predicate ref {c}")
                name = "pred:" + c
            self.bools.add(name)
            return ("atom", name)
        t = strip(c["type"])
        if t == "survives_explosion":
            return ("true",)
        if t == "location_check":
            self.warnings.add("location_check ignored")
            return ("true",)
        if t == "match_block":
            if strip(c["blocks"]) != self.block:
                self.warnings.add(f"match_block on {c['blocks']}")
            terms = []
            for prop, val in sorted(c.get("state", {}).items()):
                val = val if isinstance(val, str) else compact(val)
                self.props[prop].add(val)
                terms.append(("state", prop, val))
            return ("and", terms)
        if t == "match_tool":
            pred = c["predicate"]
            if pred == {"items": "minecraft:shears"}:
                name = "shears"
            elif set(pred) == {"items"}:
                name = "tool=" + pred["items"]
            else:
                name = "tool:" + compact(pred)
            self.bools.add(name)
            return ("atom", name)
        if t == "entity_properties":
            name = "entity_breaker" if c.get("predicate") == {} and c.get("entity") == "this" else "entity:" + compact(c)
            self.bools.add(name)
            return ("atom", name)
        if t == "table_bonus":
            if c["enchantment"] != "minecraft:fortune":
                self.warnings.add(f"table_bonus on {c['enchantment']}")
            return ("rand", tuple(c["chances"]))
        if t == "random_chance":
            return ("rand", (c["chance"],))
        if t == "inverted":
            return ("not", self.cond(c["term"]))
        if t in ("any_of", "all_of"):
            return ("or" if t == "any_of" else "and", [self.cond(x) for x in c["terms"]])
        self.warnings.add(f"unhandled condition {t}")
        self.bools.add("cond:" + t)
        return ("atom", "cond:" + t)

    def conds(self, node):
        return ("and", [self.cond(c) for c in as_list(node.get("condition"))])

    def collect(self, node):
        """Parse every condition once so all variables are known before enumerating."""
        if isinstance(node, dict):
            self.conds(node)
            for v in node.values():
                self.collect(v)
        elif isinstance(node, list):
            for v in node:
                self.collect(v)

    def find_counter_props(self, node, found):
        """State props that only feed `set_count add` (one item per attached face): not enumerated."""
        if isinstance(node, dict):
            c = node.get("condition")
            if strip(node.get("type", "")) == "set_count" and node.get("add") and isinstance(c, dict) \
                    and strip(c["type"]) == "match_block" and len(c.get("state", {})) == 1:
                found.append(next(iter(c["state"])))
            for v in node.values():
                self.find_counter_props(v, found)
        elif isinstance(node, list):
            for v in node:
                self.find_counter_props(v, found)
        return found

    def ev(self, e, a):
        """-> (holds, chance-per-fortune-level tuple or None)."""
        k = e[0]
        if k == "true":
            return True, None
        if k == "atom":
            return a[e[1]], None
        if k == "state":
            if e[1] in self.counter_props:
                raise Unknown(e[1])
            return a["state:" + e[1]] == e[2], None
        if k == "rand":
            return True, e[1]
        if k == "and":
            ok, ch = True, None
            for x in e[1]:
                o, c = self.ev(x, a)
                ok = ok and o
                ch = mul(ch, c)
            return ok, ch
        if k == "or":
            res = [self.ev(x, a) for x in e[1]]
            if any(c for _o, c in res):
                self.warnings.add("random inside any_of")
            return any(o for o, _c in res), None
        o, c = self.ev(e[1], a)
        if c:
            self.warnings.add("random inside inverted")
        return not o, None

    def count(self, mods, a):
        lo, hi, fortune, notes, faces = 1, 1, "none", [], []
        for m in mods:
            t = strip(m["type"])
            try:
                ok, ch = self.ev(self.conds(m), a)
            except Unknown as u:
                faces.append(str(u))
                hi += m["count"]
                continue
            if ch:
                self.warnings.add("random condition on a modifier")
            if not ok or t == "explosion_decay":
                continue
            if t == "set_count":
                clo, chi, note = number(m["count"])
                if note:
                    notes.append(note)
                lo, hi = (lo + clo, hi + chi) if m.get("add") else (clo, chi)
            elif t == "apply_bonus":
                if m["enchantment"] != "minecraft:fortune":
                    self.warnings.add(f"apply_bonus on {m['enchantment']}")
                f, p = strip(m["formula"]), m.get("parameters", {})
                if f == "ore_drops":
                    fortune = "ore_drops"
                elif f == "uniform_bonus_count":
                    fortune = f"uniform_bonus:{p['bonusMultiplier']}"
                elif f == "binomial_with_bonus_count":
                    fortune = f"binomial:{p['extra']},{p['probability']}"
                    notes.append(f"max includes +{p['extra']} from binomial(extra+level,p) at level 0")
                    hi += p["extra"]
                else:
                    fortune = f
                    self.warnings.add(f"unknown formula {f}")
            elif t == "limit_count":
                lim = m["limit"]
                lim = {"min": lim, "max": lim} if isinstance(lim, (int, float)) else lim
                if lo < lim.get("min", lo) or hi > lim.get("max", hi):
                    notes.append(f"count=uniform({lo},{hi}) before the clamp; min/max columns are clamped")
                if "min" in lim:
                    lo, hi = max(lo, lim["min"]), max(hi, lim["min"])
                if "max" in lim:
                    lo, hi = min(lo, lim["max"]), min(hi, lim["max"])
                notes.append(f"limit_count={lim.get('min', '')}..{lim.get('max', '')} after fortune")
            elif t == "copy_components":
                notes.append("copy_components:" + ",".join(strip(x) for x in m.get("include", [])))
            elif t == "copy_state":
                notes.append("copy_state:" + ",".join(m.get("properties", [])))
            else:
                notes.append("modifier:" + compact(m))
        if faces:
            notes.append(f"count = number of true states among {','.join(sorted(faces))} (base {lo}); min clamped to 1")
            lo = max(lo, 1)
        return lo, hi, fortune, notes

    def entry(self, e, a, remaining, outer_mods, out):
        """Emit drops of one entry; -> probability mass left for later siblings of an alternatives."""
        ok, ch = self.ev(self.conds(e), a)
        if not ok:
            return remaining
        t = strip(e["type"])
        if t == "alternatives":
            if ch:
                self.warnings.add("random condition on alternatives")
            r = remaining
            for child in e["children"]:
                r = self.entry(child, a, r, as_list(e.get("modifier")) + outer_mods, out)
                if r is ZERO:
                    break
            return r
        if t not in ("item", "dynamic"):
            self.warnings.add(f"unhandled entry {t}")
            return remaining
        name = strip(e["name"]) if t == "item" else "dynamic:" + strip(e["name"])
        lo, hi, fortune, notes = self.count(as_list(e.get("modifier")) + outer_mods, a)
        if remaining is not None:
            notes.append("else-branch: chance = 1 - chance of the earlier alternative")
        out.append((name, lo, hi, mul(remaining, ch) or (1.0,), fortune, tuple(notes)))
        return mul(remaining, tuple(1.0 - x for x in ch)) if ch else ZERO

    def drops(self, a):
        out = []
        table_mods = as_list(self.data.get("modifier"))
        for pool in self.data.get("pools", []):
            ok, ch = self.ev(self.conds(pool), a)
            if not ok:
                continue
            if ch:
                self.warnings.add("random condition on a pool")
            if pool.get("rolls") != 1 or pool.get("bonus_rolls", 0) != 0:
                self.warnings.add(f"rolls={compact(pool.get('rolls'))}")
            if len(pool["entries"]) != 1:
                self.warnings.add("pool with several weighted entries")
            for e in pool["entries"]:
                self.entry(e, a, None, as_list(pool.get("modifier")) + table_mods, out)
        return out

    def rows(self):
        """-> ([(condition, item, min, max, chance tuple, fortune, notes)], tool combos that drop anything)."""
        found = self.find_counter_props(self.data, [])
        self.counter_props = set(found) if len(found) >= 3 else set()
        self.collect(self.data)
        bools = sorted(b for b in self.bools if b not in ("silk", "shears"))
        tool_dom = [s + h for s in ("TF" if "silk" in self.bools else "x") for h in ("TF" if "shears" in self.bools else "x")]
        variables = [("tool", tool_dom)] + [(b, [True, False]) for b in bools]
        for p, used in sorted(self.props.items()):
            if p in self.counter_props:
                continue
            full = self.domains.get(p)
            if full is None or not used <= full:
                self.warnings.add(f"state domain unknown for {p}")
                full = used | {OTHER}
            variables.append(("state:" + p, sorted(full, key=sort_key)))
        groups, dropping = defaultdict(set), set()
        for combo in itertools.product(*(d for _n, d in variables)):
            a = dict(zip((n for n, _d in variables), combo))
            a["silk"], a["shears"] = combo[0][0] == "T", combo[0][1] == "T"
            seen = Counter()
            for d in self.drops(a):
                seen[d] += 1
                groups[(d, seen[d])].add(combo)
                dropping.add(combo[0])
        rows = []
        for (d, _n), combos in groups.items():
            rows += [(label,) + d for label in self.labels(variables, combos)]
        return rows, expand_tool(dropping)

    def labels(self, variables, combos):
        proj = [set(c[i] for c in combos) for i in range(len(variables))]
        size = 1
        for p in proj:
            size *= len(p)
        if size != len(combos):
            i = max(i for i, p in enumerate(proj) if len(p) > 1)
            out = []
            for v in sorted(proj[i], key=str):
                out += self.labels(variables, {c for c in combos if c[i] == v})
            return out
        parts = []
        for (name, dom), p in zip(variables, proj):
            if len(p) == len(dom):
                continue
            if name == "tool":
                parts.append(TOOL_LABELS[expand_tool(p)])
            elif name.startswith("state:"):
                rest = set(dom) - p
                negate = OTHER in p or len(rest) < len(p)
                vals = "|".join(sorted(rest if negate else p, key=sort_key))
                parts.append(name[len("state:"):] + ("!=" if negate else "=") + vals)
            else:
                parts.append(name if p == {True} else "!" + name)
        return [";".join(x for x in parts if x) or "default"]
