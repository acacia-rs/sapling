"""Transcribes the overworld feature rules, and the features they place, from BDS's vanilla data into
src/feature/data.rs.

    py -3 -I scripts/features_to_rust.py <bds> > src/feature/data.rs

Documents are read as bds_packs.documents describes. Biome filters are evaluated here against each
overworld biome's tags, as BDS does when it attaches rules to biomes at load. A rule is kept only when
every feature it reaches has a ported type; the others are listed in a comment.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from bds_packs import IDS, biomes, documents  # noqa: E402

PASSES = ['first_pass', 'before_underground_pass', 'underground_pass', 'after_underground_pass', 'before_surface_pass',
          'surface_pass', 'after_surface_pass', 'before_sky_pass', 'sky_pass', 'after_sky_pass', 'final_pass']
PORTED = {'minecraft:ore_feature'}
ORDERS = {'xyz', 'xzy', 'yxz', 'yzx', 'zxy', 'zyx'}
# Blocks whose Rust variant is not the CamelCase of their name (with states BDS flattens).
VARIANTS = {('stone', 'andesite'): 'Andesite', ('stone', 'diorite'): 'Diorite', ('stone', 'granite'): 'Granite',
            ('hardened_clay', None): 'Terracotta'}


def block(value):
    if isinstance(value, dict):
        name = value['name']
        states = value.get('states') or {}
    else:
        name, states = value, {}
    name = name.removeprefix('minecraft:')
    variant = VARIANTS.get((name, states.get('stone_type')))
    return 'Block::' + (variant or ''.join(part.title() for part in name.split('_')))


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


def expr(value):
    if isinstance(value, bool):
        return f'Expr::Const({float(value)!r})'
    if isinstance(value, (int, float)):
        return f'Expr::Const({float(value)!r})'
    try:
        return f'Expr::Const({float(value)!r})'
    except ValueError:
        return f'Expr::Molang({value!r})'.replace("'", '"') if '"' not in value else f'Expr::Molang(r#"{value}"#)'


def constant(value):
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def coordinate(value):
    if not isinstance(value, dict):
        return f'Coordinate::Fixed({expr(value if value is not None else 0)})'
    kind = value['distribution']
    lo, hi = value['extent']
    extent = f'[{expr(lo)}, {expr(hi)}]'
    if kind in ('fixed_grid', 'jittered_grid'):
        step, offset = int(value.get('step_size', 1)), int(value.get('grid_offset', 0))
        return f'Coordinate::{"Fixed" if kind == "fixed_grid" else "Jittered"}Grid {{ extent: {extent}, step: {step}, offset: {offset} }}'
    return f'Coordinate::{"".join(p.title() for p in kind.split("_"))}({extent})'


def distribution(dist):
    """The scatter parameters with BDS's load-time rewrites of bad constants."""
    iterations = dist.get('iterations', 0)
    if constant(iterations) and iterations < 0:
        iterations = 1
    chance = dist.get('scatter_chance')
    if isinstance(chance, dict):
        num, den = int(chance.get('numerator', 0)), int(chance.get('denominator', 0))
        chance = f'Chance::Fraction({num}, {den if den > num else 1})'
    else:
        if chance is None or (constant(chance) and not 0 < chance <= 100):
            chance = 100
        chance = f'Chance::Percent({expr(chance)})'
    order = dist.get('coordinate_eval_order', 'xzy')
    assert order in ORDERS, order
    axes = ', '.join(f'Axis::{c.upper()}' for c in order)
    return (f'Distribution {{ iterations: {expr(iterations)}, chance: {chance}, order: [{axes}], '
            f'x: {coordinate(dist.get("x"))}, y: {coordinate(dist.get("y"))}, z: {coordinate(dist.get("z"))} }}')


class Features:
    def __init__(self, docs):
        self.docs = {}
        for doc in docs.values():
            kind = next(k for k in doc if k.startswith('minecraft:'))
            self.docs[doc[kind]['description']['identifier'].lower()] = (kind, doc[kind])
        self.emitted = {}

    def kind(self, name):
        found = self.docs.get(name.lower())
        return found[0] if found else '(hardcoded)'

    def ported(self, name, seen=()):
        return self.kind(name) in PORTED

    def index(self, name):
        name = name.lower()
        if name not in self.emitted:
            self.emitted[name] = (len(self.emitted), self.rust(*self.docs[name]))
        return self.emitted[name][0]

    def rust(self, kind, body):
        if kind == 'minecraft:ore_feature':
            rules = ', '.join(
                f'ReplaceRule {{ places: {block(r["places_block"])}, may_replace: &[{", ".join(block(b) for b in r["may_replace"])}] }}'
                for r in body.get('replace_rules', []))
            discard = float(body.get('discard_chance_on_air_exposure', 0.0))
            return f'Feature::Ore(Ore {{ count: {int(body["count"])}, discard_on_air: {discard!r}, rules: &[{rules}] }})'
        raise SystemExit(f'{kind} is not ported')


def main():
    bds = sys.argv[1]
    tags_by_id = {}
    for name, doc in biomes(bds).items():
        if name in IDS:
            tags_by_id[IDS[name]] = set(doc['minecraft:biome']['components'].get('minecraft:tags', {}).get('tags', []))
    features = Features(documents(bds, 'features'))
    rules, skipped = [], []
    for file, doc in sorted(documents(bds, 'feature_rules').items()):
        rule = doc['minecraft:feature_rules']
        name, places = rule['description']['identifier'], rule['description']['places_feature']
        conditions = rule['conditions']
        pass_ = conditions['placement_pass']
        biome_filter = conditions.get('minecraft:biome_filter')
        if pass_ not in PASSES or not any(accepts(biome_filter, tags) for tags in tags_by_id.values()):
            continue
        if not features.ported(places):
            skipped.append(f'{name} ({features.kind(places)})')
            continue
        rules.append((PASSES.index(pass_), name, pass_, biome_filter, places, rule.get('distribution', {'iterations': 0})))
    print(f'{len(rules)} rules transcribed, {len(skipped)} with unported features skipped:', file=sys.stderr)
    for line in skipped:
        print(f'  {line}', file=sys.stderr)
    print('// Generated by scripts/features_to_rust.py from BDS 1.26.52 vanilla behaviour packs.')
    print('use super::spec::*;')
    print('use crate::block::Block;')
    print()
    print('pub static RULES: &[Rule] = &[')
    for _, name, pass_, biome_filter, places, dist in sorted(rules, key=lambda r: r[0]):
        pass_rust = ''.join(p.title() for p in pass_.removesuffix('_pass').split('_'))
        print(f'    Rule {{ name: "{name}", pass: Pass::{pass_rust}, biomes: {biome_set(biome_filter, tags_by_id)}, '
              f'feature: {features.index(places)},')
        print(f'        distribution: {distribution(dist)} }},')
    print('];')
    print()
    print('pub static FEATURES: &[Feature] = &[')
    for name, (index, rust) in sorted(features.emitted.items(), key=lambda kv: kv[1][0]):
        print(f'    {rust}, // {index} {name}')
    print('];')


main()
