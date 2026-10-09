"""Splits the oracle's stable dripstone errors into BDS's nondeterminism floor and deterministic desyncs.

usage: py -3 dripstone_floor.py <trace> <dump.tsv>...   (SHOW=<cx>,<cz> lists that chunk's deterministic cells)

<trace> is the stderr of `examples/decorate_trace.rs` run with SAPLING_TRACE_RULES=dripstone on a tree with
scripts/data/dripstone_trace.patch applied (FEAT, CLUSTER, POINTED and W lines); the dumps are SAPLING_ORACLE_DUMP's.
Per (source chunk, rule) the features run in order on one generator. Each wrong cell gets an owner: the feature of
ours that wrote it, else the smallest feature box holding it. A sequence's first owner is its first desync; it is
floor when a trigger lies in that feature's scan box (strict) or an earlier feature's (lenient): a lichen or sculk
difference (ours vs BDS, or BDS vs its twin), a dripstone cell the twin disagrees on, or an error owned by an
earlier-decorated sequence already judged floor. ROOT marks deterministic desyncs with no earlier sequence's
error in the box.
"""
import collections
import sys

THREAD_LOCAL = ('glow_lichen', 'sculk')


def name(cell):
    return cell.split('[', 1)[0]


def cube(at, dx, dy, dz):
    return (at[0] - dx, at[1] - dy, at[2] - dz, at[0] + dx, at[1] + dy, at[2] + dz)


def parse_trace(path):
    seqs = collections.defaultdict(list)
    last = None
    for line in open(path):
        p = line.rstrip('\n').split('\t')
        if p[0] == 'FEAT':
            last = (int(p[1]), int(p[2]), p[3])
            at = tuple(map(int, p[5:8]))
            if 'large' in last[2]:
                seqs[last].append({'at': at, 'own': cube(at, 18, 18, 18), 'scan': cube(at, 18, 18, 18), 'kind': 'large'})
            elif 'cluster' in last[2]:
                seqs[last].append({'at': at, 'own': None, 'scan': None, 'kind': 'cluster'})
        elif p[0] == 'CLUSTER' and last and 'cluster' in last[2]:
            f = seqs[last][-1]
            f['own'] = f['scan'] = cube(f['at'], int(p[1]), 12, int(p[2]))
        elif p[0] == 'POINTED' and last and 'small' in last[2]:
            at = tuple(map(int, p[1:4]))
            seqs[last].append({'at': at, 'own': cube(at, 3, 3, 3), 'scan': cube(at, 12, 3, 12), 'kind': 'pointed'})
        elif p[0] == 'W' and last and seqs[last]:
            WRITER[tuple(map(int, p[1:4]))] = (last, len(seqs[last]) - 1)
    return seqs


WRITER = {}
SIZE = {'pointed': 0, 'cluster': 1, 'large': 2}


def inside(box, p):
    return box is not None and all(box[i] <= p[i] <= box[i + 3] for i in range(3))


class Grid:
    def __init__(self, points):
        self.cells = collections.defaultdict(list)
        for q in points:
            self.cells[(q[0] >> 4, q[2] >> 4)].append(q)

    def within(self, box):
        if box is None:
            return []
        return [q for bx in range(box[0] >> 4, (box[3] >> 4) + 1) for bz in range(box[2] >> 4, (box[5] >> 4) + 1)
                for q in self.cells.get((bx, bz), ()) if inside(box, q)]


def load(paths):
    errors, triggers = [], []
    for path in paths:
        with open(path) as f:
            head = f.readline().rstrip('\n').split('\t')
            for line in f:
                r = dict(zip(head, line.rstrip('\n').split('\t')))
                if r.get('stage') == 'L':
                    continue
                p = (int(r['x']), int(r['y']), int(r['z']))
                names = [name(r[k]) for k in ('ours', 'bds', 'twin')]
                drip = any('dripstone' in n for n in names)
                if any(any(t in n for t in THREAD_LOCAL) for n in names) or drip and r['stable'] == 'U':
                    triggers.append(p)
                if drip and r['stable'] == 'S':
                    errors.append(p)
    return errors, triggers


def main():
    seqs = parse_trace(sys.argv[1])
    errors, triggers = load(sys.argv[2:])
    egrid, tgrid = Grid(errors), Grid(triggers)
    owners = collections.defaultdict(list)
    for key, feats in seqs.items():
        for i, f in enumerate(feats):
            for e in egrid.within(f['own']):
                owners[e].append((key, i))
    # Our own writer if we placed it, else the smallest feature whose box holds it.
    owner = {}
    for e in errors:
        cands = [WRITER[e]] if e in WRITER else sorted(owners[e], key=lambda o: (SIZE[seqs[o[0]][o[1]]['kind']], o[1]))[:1]
        if cands:
            owner[e] = cands[0]
    first = {}
    for k, i in owner.values():
        first[k] = min(first.get(k, i), i)
    # Sequences in decoration order: earlier phases first, then rule order within a chunk. A desync is floor
    # when a trigger, or an error left by an earlier sequence's floor feature, lies in its scan box; `root`
    # marks deterministic desyncs with no earlier sequence's error in the box (first causes).
    order = sorted(first, key=lambda k: (PHASE[(k[0] & 1) | (k[1] & 1) << 1], RULES.index(k[2].split(':')[1]), k))
    rank = {k: n for n, k in enumerate(order)}
    verdicts = {}
    for key in order:
        i, feats = first[key], seqs[key]

        def earlier(box):
            cells = [owner[e] for e in egrid.within(box) if e in owner]
            return [(k, j) for k, j in cells if k != key and rank.get(k, -1) < rank[key] and j >= first[k]]

        def floor_in(box):
            return bool(tgrid.within(box)) or any(verdicts[k][2] for k, _ in earlier(box))

        strict = floor_in(feats[i]['scan'])
        lenient = strict or any(floor_in(g['scan']) for g in feats[:i])
        root = not earlier(feats[i]['scan'])
        verdicts[key] = (i, strict, lenient, feats[i], root)
    counts = collections.Counter()
    blame = collections.Counter()
    for e in errors:
        if e not in owner:
            counts['unattributed'] += 1
            continue
        k, i = owner[e]
        v = verdicts.get(k)
        if v is None or i < v[0]:
            counts['before desync'] += 1
        elif v[1]:
            counts['floor strict'] += 1
        elif v[2]:
            counts['floor lenient'] += 1
        else:
            counts['deterministic'] += 1
            blame[k] += 1
            if SHOW and SHOW == f'{k[0]},{k[1]}':
                print('cell', e, 'feature', i, seqs[k][i]['at'], 'written by us' if e in WRITER else '')
    print(f'errors {len(errors)} triggers {len(triggers)}', dict(counts))
    det = [(k, v) for k, v in verdicts.items() if not v[2]]
    print(f'sequences desynced {len(verdicts)}, deterministic {len(det)}, roots {sum(v[4] for _, v in det)}')
    for (cx, cz, rule), (i, _, _, f, root) in sorted(det, key=lambda kv: -blame[kv[0]])[:30]:
        print(f'  {cx},{cz} {rule.split(":")[1]} #{i} {f["kind"]} at {f["at"]} own {f["own"]} cells {blame[(cx, cz, rule)]}{" ROOT" if root else ""}')


import os
SHOW = os.environ.get('SHOW')
PHASE = [0, 1, 3, 2]
RULES = ['surface_pass_large_dripstone_feature', 'dripstone_cluster_feature', 'large_dripstone_feature', 'small_dripstone_feature']


main()
