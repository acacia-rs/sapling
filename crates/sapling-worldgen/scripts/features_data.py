"""The features half of features_to_rust.py: each feature document (or hardcoded feature) as the Rust
`Feature` of src/feature/spec.rs, the features it places first, and the scatter parameters they share
with rules. A feature is ported when its type is and every feature it places is."""
import os
import re

ORDERS = {'xyz', 'xzy', 'yxz', 'yzx', 'zxy', 'zyx'}
PORTED = {'minecraft:ore_feature', 'minecraft:scatter_feature', 'minecraft:aggregate_feature', 'minecraft:sequence_feature',
          'minecraft:weighted_random_feature', 'minecraft:single_block_feature'}
# Features BDS builds in code (registered in 1482f4580), with the parameters it passes them.
_SOIL = 'needs: &[Block::Water], replaces: &[Block::Dirt, Block::CoarseDirt, Block::GrassBlock]'
# 1482f4580 sizes these from a flag (kelp/seagrass 1 or 10 tries, flowers/mushrooms/berries 64 or 1)
# that could not be read statically; the oracle fits 1 try for all of them.
_KELP_COUNT = 1
_PATCH_COUNT = 1
_LEGACY = 'Feature::Legacy(Legacy::{})'
HARDCODED = {
    'minecraft:sand_feature': f'Feature::Disk(Disk {{ block: Block::Sand, radius: 7, half_height: 2, {_SOIL} }})',
    'minecraft:gravel_feature': f'Feature::Disk(Disk {{ block: Block::Gravel, radius: 6, half_height: 2, {_SOIL} }})',
    'minecraft:clay_feature': 'Feature::Disk(Disk { block: Block::Clay, radius: 4, half_height: 1, '
                              'needs: &[Block::Water, Block::Clay], replaces: &[Block::Dirt, Block::CoarseDirt, Block::Mud] })',
    'minecraft:kelp_feature': f'Feature::Kelp {{ count: {_KELP_COUNT} }}',
    'minecraft:seagrass_feature': f'Feature::Seagrass {{ count: {_KELP_COUNT}, tall: 0.6 }}',
    'minecraft:shortest_seagrass_feature': 'Feature::Seagrass { count: 1, tall: 0.3 }',
    'minecraft:shorter_seagrass_feature': 'Feature::Seagrass { count: 1, tall: 0.4 }',
    'minecraft:taller_seagrass_feature': 'Feature::Seagrass { count: 1, tall: 0.8 }',
    'minecraft:dead_bush_feature': _LEGACY.format('DeadBush'),
    'minecraft:reeds_feature': _LEGACY.format('Reeds'),
    'minecraft:cactus_feature': _LEGACY.format('Cactus'),
    'minecraft:melon_feature': _LEGACY.format('Melon'),
    'minecraft:waterlily_feature': _LEGACY.format('Waterlily'),
    'minecraft:pumpkin_feature': _LEGACY.format('Pumpkin { count: 96 }'),
    'minecraft:sweet_berry_bush_feature': _LEGACY.format(f'SweetBerries {{ count: {_PATCH_COUNT} }}'),
    'minecraft:brown_mushroom_feature': _LEGACY.format(f'Mushroom {{ block: Block::BrownMushroom, count: {_PATCH_COUNT} }}'),
    'minecraft:red_mushroom_feature': _LEGACY.format(f'Mushroom {{ block: Block::RedMushroom, count: {_PATCH_COUNT} }}'),
    'minecraft:legacy:overworld_flower_feature': _LEGACY.format(f'Flower {{ kind: FlowerKind::Overworld, count: {_PATCH_COUNT} }}'),
    'minecraft:legacy:plains_flower_feature': _LEGACY.format(f'Flower {{ kind: FlowerKind::Plains, count: {_PATCH_COUNT} }}'),
    'minecraft:legacy:swamp_flower_feature': _LEGACY.format(f'Flower {{ kind: FlowerKind::Swamp, count: {_PATCH_COUNT} }}'),
}
EARLY_OUT = {'none': 'EarlyOut::None', 'first_failure': 'EarlyOut::FirstFailure', 'first_success': 'EarlyOut::FirstSuccess'}
# Old names and states BDS flattens into the blocks of src/block.rs.
ALIASES = {('stone', 'andesite'): 'andesite', ('stone', 'diorite'): 'diorite', ('stone', 'granite'): 'granite',
           ('tallgrass', 'tall'): 'short_grass', ('tallgrass', 'default'): 'short_grass', ('tallgrass', 'fern'): 'fern'}


class NotPorted(Exception):
    pass


def _rust_blocks():
    """(name, states) -> variant, from the `blocks!` table of src/block.rs."""
    path = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'src', 'block.rs')
    text = open(path, encoding='utf-8').read()
    return {(m[2], m[3]): m[1] for m in re.finditer(r'^\s*(\w+) => "([a-z_]+)" "([^"]*)",', text, re.M)}


BLOCKS = _rust_blocks()


def block(value):
    """The Rust block for a JSON block reference; NotPorted when src/block.rs lacks it."""
    name, states = (value['name'], value.get('states') or {}) if isinstance(value, dict) else (value, {})
    name = name.removeprefix('minecraft:')
    flattened = next((v for k, v in states.items() if k in ('stone_type', 'tall_grass_type')), None)
    name = ALIASES.get((name, flattened), name)
    states = {k: v for k, v in states.items() if k not in ('stone_type', 'tall_grass_type')}
    text = ','.join(f'{k}={int(v) if isinstance(v, bool) else v}' for k, v in states.items())
    variant = BLOCKS.get((name, text))
    if variant is None and not text:
        variant = next((v for (n, _), v in BLOCKS.items() if n == name), None)
    if variant is None:
        raise NotPorted(f'block {name} [{text}]')
    return 'Block::' + variant


def blocks(values):
    """A block list that may name blocks sapling never generates (dropped: nothing can match them)."""
    out = []
    for value in values if isinstance(values, list) else [values]:
        try:
            out.append(block(value))
        except NotPorted:
            pass
    return '&[' + ', '.join(out) + ']'


def expr(value):
    if isinstance(value, (bool, int, float)):
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

    def ported(self, name):
        """Whether `name` transcribes, emitting it if so; a failed attempt emits nothing."""
        saved = dict(self.emitted)
        try:
            self.index(name)
            return True
        except NotPorted:
            self.emitted = saved
            return False

    def index(self, name):
        name = name.lower()
        if name not in self.emitted:
            index = len(self.emitted)
            if name in HARDCODED:
                self.emitted[name] = (index, HARDCODED[name])
            elif name in self.docs and self.docs[name][0] in PORTED:
                self.emitted[name] = (index, None)
                self.emitted[name] = (index, self.rust(*self.docs[name]))
            else:
                raise NotPorted(name)
        return self.emitted[name][0]

    def rust(self, kind, body):
        if kind == 'minecraft:ore_feature':
            rules = ', '.join(
                f'ReplaceRule {{ places: {block(r["places_block"])}, may_replace: {blocks(r["may_replace"])} }}'
                for r in body.get('replace_rules', []))
            discard = float(body.get('discard_chance_on_air_exposure', 0.0))
            return f'Feature::Ore(Ore {{ count: {int(body["count"])}, discard_on_air: {discard!r}, rules: &[{rules}] }})'
        if kind == 'minecraft:scatter_feature':
            dist = body.get('distribution', body)
            floor = 'true' if body.get('project_input_to_floor', False) else 'false'
            return (f'Feature::Scatter(Scatter {{ distribution: {distribution(dist)}, project_to_floor: {floor}, '
                    f'feature: {self.index(body["places_feature"])} }})')
        if kind in ('minecraft:aggregate_feature', 'minecraft:sequence_feature'):
            children = ', '.join(str(self.index(f)) for f in body['features'])
            variant = 'Aggregate' if kind == 'minecraft:aggregate_feature' else 'Sequence'
            return f'Feature::{variant}(&[{children}], {EARLY_OUT[body.get("early_out", "none")]})'
        if kind == 'minecraft:weighted_random_feature':
            children = ', '.join(f'({self.index(f)}, {float(w)!r})' for f, w in body['features'])
            return f'Feature::WeightedRandom(&[{children}])'
        if kind == 'minecraft:single_block_feature':
            return self.single_block(body)
        raise NotPorted(kind)

    def single_block(self, body):
        if body.get('may_attach_to') or body.get('randomize_rotation'):
            raise NotPorted('single_block_feature with may_attach_to or randomize_rotation')
        not_attach = body.get('may_not_attach_to') or {}
        if set(not_attach) - {'bottom'}:
            raise NotPorted('may_not_attach_to beyond bottom')
        places = body['places_block']
        entries = places if isinstance(places, list) else [{'block': places, 'weight': 1}]
        weighted = ', '.join(f'({block(e["block"])}, {int(e.get("weight", 1))})' for e in entries)
        flag = lambda key: 'true' if body.get(key, False) else 'false'  # noqa: E731
        return (f'Feature::SingleBlock(SingleBlock {{ blocks: &[{weighted}], may_replace: {blocks(body.get("may_replace", []))}, '
                f'placement: {flag("enforce_placement_rules")}, survival: {flag("enforce_survivability_rules")}, '
                f'not_on: {blocks(not_attach.get("bottom", []))} }})')
