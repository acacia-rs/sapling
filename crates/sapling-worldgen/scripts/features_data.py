"""The features half of features_to_rust.py: each feature document (or hardcoded feature) as the Rust
`Feature` of src/feature/spec.rs, the features it places first, and the scatter parameters they share
with rules. A feature is ported when its type is and every feature it places is."""
import biome_expr
import cave_data
import legacy_data
import trees_data
from blocks_data import NotPorted, block, blocks

ORDERS = {'xyz', 'xzy', 'yxz', 'yzx', 'zxy', 'zyx'}
PORTED = {'minecraft:ore_feature', 'minecraft:scatter_feature', 'minecraft:aggregate_feature', 'minecraft:sequence_feature',
          'minecraft:weighted_random_feature', 'minecraft:single_block_feature', 'minecraft:tree_feature', 'minecraft:search_feature',
          'minecraft:geode_feature'} | cave_data.PORTED
_HANDWRITTEN = {**legacy_data.LEGACY, **cave_data.LEGACY}
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
    'minecraft:legacy:flower_forest_flower_feature': _LEGACY.format(f'Flower {{ kind: FlowerKind::FlowerForest, count: {_PATCH_COUNT} }}'),
    'minecraft:legacy:ice_feature': _LEGACY.format('Iceberg'),
    'minecraft:legacy:springs_feature': _LEGACY.format('Springs'),
    'minecraft:ice_patch_feature': _LEGACY.format('IcePatch'),
    'minecraft:ice_spike_feature': _LEGACY.format('IceSpike'),
    'minecraft:pickle_feature': _LEGACY.format('SeaPickle'),
    # Speleothems: blocks, cluster heights and wetness (mean, deviation, min, max) from 1482f4580.
    'minecraft:pointed_dripstone_feature': 'Feature::Speleothem(Speleothem::Pointed(&DRIPSTONE))',
    'minecraft:sulfur_spike_feature': 'Feature::Speleothem(Speleothem::Pointed(&SULFUR))',
    'minecraft:dripstone_cluster_feature':
        'Feature::Speleothem(Speleothem::Cluster { blocks: &DRIPSTONE, height: [3, 5], wetness: [0.1, 0.3, 0.1, 1.0] })',
    'minecraft:sulfur_spike_cluster_feature':
        'Feature::Speleothem(Speleothem::Cluster { blocks: &SULFUR, height: [1, 4], wetness: [0.0, 0.0, 0.0, 0.0] })',
    'minecraft:large_dripstone_feature': 'Feature::Speleothem(Speleothem::Large(&DRIPSTONE))',
    # LakeFeature(water, sulfur, [sulfur_spike]) in 1482f4580.
    'minecraft:sulfur_pool': 'Feature::Pool(Pool { liquid: Block::Water, border: Block::Sulfur, avoid: &[Block::SulfurSpike] })',
    'minecraft:bamboo_feature': _LEGACY.format('Bamboo'),
    'minecraft:podzol_area_feature': _LEGACY.format('PodzolArea'),
    'minecraft:vines_feature': _LEGACY.format('Vines'),
    'minecraft:pale_moss_vegetation_feature': _LEGACY.format('PaleMossVegetation'),
    'minecraft:eyeblossom_feature': _LEGACY.format('Eyeblossom'),
    'minecraft:meadow_flower_feature': _LEGACY.format('MeadowFlower'),
    'minecraft:grass_disc_feature': _LEGACY.format('GrassDisc'),
    # The village piles: one class (vtable 14a9258d0) over the block 1482f4580 passes.
    'minecraft:hay_pile_feature': _LEGACY.format('Pile(PileBlock::Hay)'),
    'minecraft:melon_pile_feature': _LEGACY.format('Pile(PileBlock::Melon)'),
    'minecraft:pumpkin_pile_feature': _LEGACY.format('Pile(PileBlock::Pumpkin)'),
    'minecraft:snow_pile_feature': _LEGACY.format('Pile(PileBlock::Snow)'),
    'minecraft:ice_pile_feature': _LEGACY.format('Pile(PileBlock::Ice)'),
}
EARLY_OUT = {'none': 'EarlyOut::None', 'first_failure': 'EarlyOut::FirstFailure', 'first_success': 'EarlyOut::FirstSuccess'}
SEARCH_LOOPS = {'-x': ('x-', 'y+', 'z-'), '+x': ('x+', 'y+', 'z+'), '-y': ('y-', 'z+', 'x-'), '+y': ('y+', 'z+', 'x+'),
                '-z': ('z-', 'y+', 'x+'), '+z': ('z+', 'y+', 'x-')}


def expr(value):
    if isinstance(value, (bool, int, float)):
        return f'Expr::Const({float(value)!r})'
    try:
        return f'Expr::Const({float(value)!r})'
    except ValueError:
        if 'has_biome_tag' in value:
            return biome_expr.by_biome(value)
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

    def unported(self):
        """The placeholder of rules whose feature is not ported."""
        if '(unported)' not in self.emitted:
            self.emitted['(unported)'] = (len(self.emitted), 'Feature::Unported')
        return self.emitted['(unported)'][0]

    def index(self, name):
        name = name.lower()
        if name not in self.emitted:
            index = len(self.emitted)
            if name in HARDCODED:
                self.emitted[name] = (index, HARDCODED[name])
            elif name in _HANDWRITTEN:
                self.emitted[name] = (index, None)
                self.emitted[name] = (index, _HANDWRITTEN[name](self.index))
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
        if kind == 'minecraft:tree_feature':
            return trees_data.tree(body, self.index)
        if kind == 'minecraft:search_feature':
            volume = body['search_volume']
            axes = {'x': 0, 'y': 1, 'z': 2}
            loops = ', '.join(f'(Axis::{a[0].upper()}, {1 if a[1] == "+" else -1}, [{int(volume["min"][axes[a[0]]])}, '
                              f'{int(volume["max"][axes[a[0]]])}])' for a in SEARCH_LOOPS[body['search_axis']])
            return (f'Feature::Search(Search {{ feature: {self.index(body["places_feature"])}, loops: [{loops}], '
                    f'required: {int(body.get("required_successes", 1))} }})')
        if kind == 'minecraft:geode_feature':
            return self.geode(body)
        if kind in cave_data.PORTED:
            return cave_data.rust(kind, body, self.index)
        raise NotPorted(kind)

    def geode(self, body):
        layers = ', '.join(f'{field}: {block(body[key])}' for field, key in (
            ('filler', 'filler'), ('inner', 'inner_layer'), ('alternate_inner', 'alternate_inner_layer'),
            ('middle', 'middle_layer'), ('outer', 'outer_layer')))
        span = lambda key: f'[{int(body["min_" + key])}, {int(body["max_" + key])}]'  # noqa: E731
        num = lambda key: repr(float(body[key]))  # noqa: E731
        return (f'Feature::Geode(Geode {{ {layers}, placements: {blocks(body.get("inner_placements", []))}, '
                f'wall: {span("outer_wall_distance")}, points: {span("distribution_points")}, '
                f'point_offset: {span("point_offset")}, max_radius: {int(body["max_radius"])}, '
                f'crack_point_offset: {num("crack_point_offset")}, noise_multiplier: {num("noise_multiplier")}, '
                f'potential_placements_chance: {num("use_potential_placements_chance")}, '
                f'alternate_chance: {num("use_alternate_layer0_chance")}, '
                f'placements_require_alternate: {str(body["placements_require_layer0_alternate"]).lower()}, '
                f'invalid_blocks_threshold: {int(body["invalid_blocks_threshold"])} }})')

    def single_block(self, body):
        if body.get('randomize_rotation'):
            raise NotPorted('single_block_feature with randomize_rotation')
        not_attach = body.get('may_not_attach_to') or {}
        if set(not_attach) - {'bottom'}:
            raise NotPorted('may_not_attach_to beyond bottom')
        places = body['places_block']
        entries = places if isinstance(places, list) else [{'block': places, 'weight': 1}]
        weighted = ', '.join(f'({block(e["block"])}, {int(e.get("weight", 1))})' for e in entries)
        flag = lambda key: 'true' if body.get(key, False) else 'false'  # noqa: E731
        return (f'Feature::SingleBlock(SingleBlock {{ blocks: &[{weighted}], may_replace: {blocks(body.get("may_replace", []))}, '
                f'placement: {flag("enforce_placement_rules")}, survival: {flag("enforce_survivability_rules")}, '
                f'not_on: {blocks(not_attach.get("bottom", []))}, attach: {attach(body.get("may_attach_to"))} }})')


def attach(value):
    if not value:
        return 'None'
    lists = {key: trees_data.block_set(value.get(key, [])) for key in
             ('top', 'bottom', 'diagonal', 'north', 'east', 'south', 'west', 'sides', 'all')}
    return (f'Some(&Attach {{ top: {lists["top"]}, bottom: {lists["bottom"]}, diagonal: {lists["diagonal"]}, '
            f'side: [{lists["north"]}, {lists["east"]}, {lists["south"]}, {lists["west"]}], sides: {lists["sides"]}, all: {lists["all"]}, '
            f'min_sides: {int(value.get("min_sides_must_attach", 4))}, auto_rotate: {str(value.get("auto_rotate", True)).lower()} }})')
