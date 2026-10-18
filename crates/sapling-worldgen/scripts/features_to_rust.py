"""Transcribes the overworld feature rules, and the features they place, from BDS's vanilla data into
src/feature/data/{rules,features}.rs.

    py -3 -I scripts/features_to_rust.py <bds> src/feature/data

Documents are read as bds_packs.documents describes. Biome filters are evaluated here against each
overworld biome's tags, as BDS does when it attaches rules to biomes at load. A rule reaching a feature
that is not ported (features_data.py) places nothing but keeps its scatter; such rules are listed on stderr.
The features of the jigsaw pools' feature elements (src/structure/jigsaw/pools*.txt) follow, named in
`ELEMENTS`.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import biome_expr  # noqa: E402
from bds_packs import END_IDS, IDS as OVERWORLD_IDS, NETHER_IDS, biomes, documents, load_order  # noqa: E402

# Rules attach to biomes by tag at load, the Nether's too.
IDS = {**OVERWORLD_IDS, **NETHER_IDS, **END_IDS}
from features_data import Features, distribution  # noqa: E402

PASSES = ['first_pass', 'before_underground_pass', 'underground_pass', 'after_underground_pass', 'before_surface_pass',
          'surface_pass', 'after_surface_pass', 'before_sky_pass', 'sky_pass', 'after_sky_pass', 'final_pass']


def tested(test, tags):
    assert test.get('test') == 'has_biome_tag' and test.get('subject', 'self') == 'self', test
    has = test['value'] in tags
    return has if test.get('operator', '==') in ('==', 'equals') else not has


def accepts(filter_, tags):
    """`minecraft:biome_filter`: a list is all_of; objects are a test or one combinator."""
    if filter_ is None:
        return True
    if isinstance(filter_, list):
        return all(accepts(f, tags) for f in filter_)
    if 'all_of' in filter_:
        return all(accepts(f, tags) for f in filter_['all_of'])
    if 'any_of' in filter_:
        return any(accepts(f, tags) for f in filter_['any_of'])
    if 'none_of' in filter_:
        return not any(accepts(f, tags) for f in filter_['none_of'])
    return tested(filter_, tags)


def biome_set(filter_, tags_by_id):
    words = [0, 0, 0, 0]
    for biome_id, tags in tags_by_id.items():
        if accepts(filter_, tags):
            words[biome_id >> 6] |= 1 << (biome_id & 63)
    return 'BiomeSet([' + ', '.join(f'{w:#x}' for w in words) + '])'


def hashed_string(text):
    """Bedrock's HashedString: 64-bit FNV-1 (multiply, then xor)."""
    h = 0xCBF29CE484222325
    for byte in text.encode():
        h = (h * 0x100000001B3 & 0xFFFFFFFFFFFFFFFF) ^ byte
    return h


class MsvcMap:
    """The iteration order of an MSVC std::unordered_map with unique keys: a key whose bucket is empty goes to
    the list's end, else in front of its bucket's first node; 8 buckets, load factor 1, x8 growth below 512
    (x2 above), a rehash re-placing the old list in order."""

    def __init__(self):
        self.keys, self.buckets, self.first = [], 8, {}

    def _place(self, key):
        bucket = hashed_string(key) & (self.buckets - 1)
        if bucket in self.first:
            self.keys.insert(self.keys.index(self.first[bucket]), key)
        else:
            self.keys.append(key)
        self.first[bucket] = key

    def insert(self, key):
        if key in self.keys:
            return False
        if len(self.keys) + 1 > self.buckets:
            self.buckets *= 8 if self.buckets < 512 else 2
            old, self.keys, self.first = self.keys, [], {}
            for k in old:
                self._place(k)
        self._place(key)
        return True


def rule_order(bds):
    """Rules as BDS attaches them to a biome: its store is an unordered_map of pass to an unordered_map of
    identifier (HashedString) to rule, filled in load order with the first of an identifier kept; attaching walks
    passes, then each pass's rules, in map order. Within a pass the decoration order is the inner map's."""
    store = {}
    for doc in load_order(bds, 'feature_rules'):
        rule = doc['minecraft:feature_rules']
        inner = store.setdefault(rule['conditions']['placement_pass'], (MsvcMap(), {}))
        if inner[0].insert(rule['description']['identifier']):
            inner[1][rule['description']['identifier']] = rule
    for keys, by_name in store.values():
        for key in keys.keys:
            yield by_name[key]


def main():
    bds = sys.argv[1]
    tags_by_id = {}
    for name, doc in biomes(bds).items():
        if name in IDS:
            tags_by_id[IDS[name]] = set(doc['minecraft:biome']['components'].get('minecraft:tags', {}).get('tags', []))
    biome_expr.TAGS.update(tags_by_id)
    features = Features(documents(bds, 'features'))
    rules, skipped = [], []
    for rule in rule_order(bds):
        name, places = rule['description']['identifier'], rule['description']['places_feature']
        conditions = rule['conditions']
        pass_ = conditions['placement_pass']
        biome_filter = conditions.get('minecraft:biome_filter')
        if pass_ not in PASSES or not any(accepts(biome_filter, tags) for tags in tags_by_id.values()):
            continue
        if not features.ported(places):
            # Kept with a placeholder: its scatter still draws the chunk's shared math.random.
            skipped.append(f'{name} ({features.kind(places)})')
            places = None
        rules.append((PASSES.index(pass_), name, pass_, biome_filter, places, rule.get('distribution', {'iterations': 0})))
    print(f'{len(rules)} rules transcribed, {len(skipped)} of them with an unported feature placed as nothing:', file=sys.stderr)
    for line in skipped:
        print(f'  {line}', file=sys.stderr)
    header = ['// Generated by scripts/features_to_rust.py from BDS 1.26.52 vanilla behaviour packs.', 'use crate::feature::spec::*;']
    lines = header + ['', 'pub static RULES: &[Rule] = &[']
    for _, name, pass_, biome_filter, places, dist in sorted(rules, key=lambda r: r[0]):
        pass_rust = ''.join(p.title() for p in pass_.removesuffix('_pass').split('_'))
        lines.append(f'    Rule {{ name: "{name}", pass: Pass::{pass_rust}, biomes: {biome_set(biome_filter, tags_by_id)}, '
                     f'feature: {features.index(places) if places else features.unported()}, distribution: {distribution(dist)} }},')
    write(os.path.join(sys.argv[2], 'rules.rs'), lines + ['];'])
    elements = []
    for name in element_features():
        if features.ported(f'minecraft:{name}'):
            elements.append(f'    ("{name}", {features.index(f"minecraft:{name}")}),')
        else:
            print(f'  element feature {name} not ported', file=sys.stderr)
    lines = header + ['use crate::feature::tree::spec::*;', 'use crate::block::Block;', '', 'pub static FEATURES: &[Feature] = &[']
    for name, (index, rust) in sorted(features.emitted.items(), key=lambda kv: kv[1][0]):
        lines.append(f'    {rust}, // {index} {name}')
    lines += ['];', '', '/// The jigsaw pools\' feature elements: feature name, index.', 'pub static ELEMENTS: &[(&str, usize)] = &[']
    write(os.path.join(sys.argv[2], 'features.rs'), lines + elements + ['];'])


def element_features():
    pools = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'src', 'structure', 'jigsaw')
    names = []
    for file in ('pools.txt', 'pools_v2.txt'):
        for line in open(os.path.join(pools, file), encoding='utf-8'):
            parts = line.split()
            if len(parts) == 4 and parts[0] == 'e' and parts[2] == 'f' and parts[3] not in names:
                names.append(parts[3])
    return names


def write(path, lines):
    with open(path, 'w', encoding='utf-8', newline='\n') as out:
        out.write('\n'.join(lines) + '\n')


main()
