"""The tree half of features_data.py: a `minecraft:tree_feature` document as the Rust `Tree` of
src/feature/tree/spec.rs. Block lists resolve tags against the blocks sapling places."""
from blocks_data import BLOCKS, NotPorted, block

# Vanilla block tags BDS hardcodes, as far as sapling places their blocks.
TAGS = {
    'dirt': ['dirt', 'grass_block', 'podzol', 'coarse_dirt', 'mycelium', 'mud', 'muddy_mangrove_roots', 'dirt_with_roots',
             'moss_block', 'farmland', 'pale_moss_block'],
    'minecraft:leaves': [n for (n, _) in BLOCKS if n.endswith('_leaves') or n == 'azalea_leaves_flowered'],
}
STEPS = {'down': 'Step::Down', 'up': 'Step::Up', 'outward': 'Step::Outward'}


def names(value):
    if isinstance(value, dict) and 'tags' in value:
        tag = value['tags'].removeprefix("query.any_tag('").removesuffix("')")
        return [(n, None) for n in TAGS[tag]]
    if isinstance(value, dict):
        return [(value['name'].removeprefix('minecraft:'), value.get('states'))]
    return [(value.removeprefix('minecraft:'), None)]


def block_set(values):
    """Every variant of each named block (any states unless the descriptor gives some); unknown names drop."""
    out = []
    for value in values if isinstance(values, list) else [values]:
        for name, states in names(value):
            if states:
                try:
                    out.append(block({'name': name, 'states': states}))
                except NotPorted:
                    pass
            else:
                out += ['Block::' + v for (n, _), v in BLOCKS.items() if n == name]
    return '&[' + ', '.join(dict.fromkeys(out)) + ']'


def axis(value, along):
    name = names(value)[0][0]
    return 'Block::' + BLOCKS[(name, f'pillar_axis={along}')]


def rng(value, default=None):
    if value is None:
        value = default
    if isinstance(value, (int, float)):
        return f'Range({int(value)}, {int(value)})'
    if isinstance(value, list):
        return f'Range({int(value[0])}, {int(value[1])})'
    lo = value.get('range_min', value.get('min'))
    hi = value.get('range_max', value.get('max'))
    return f'Range({int(lo)}, {int(hi)})'


def chance(value):
    if value is None:
        return 'Odds::Percent(0.0)'
    if isinstance(value, dict):
        return f'Odds::Fraction({int(value["numerator"])}, {int(value["denominator"])})'
    return f'Odds::Percent({float(value)!r})'


def decorative(value):
    name = names(value)[0][0]
    return 'Decorative::Vine' if name == 'vine' else f'Decorative::Block({block(value)})'


def decoration(value):
    if value is None:
        return 'None'
    if 'decoration_blocks_sequence' in value:
        entries = ', '.join(f'({decorative(e["block"])}, {rng(e.get("count"), 1)})' for e in value['decoration_blocks_sequence'])
    else:
        steps = int(value.get('num_steps', 0)) + 1
        entries = f'({decorative(value["decoration_block"])}, Range({steps}, {steps}))'
    step = STEPS[value.get('step_direction', 'down')]
    return f'Some(Decoration {{ entries: &[{entries}], chance: {chance(value.get("decoration_chance"))}, step: {step} }})'


def weighted_blocks(pairs):
    return '&[' + ', '.join(f'({block(b)}, {int(w)})' for b, w in pairs) + ']'


def canopy(body):
    """The one `*canopy` key of a tree or branch, as `Some(&Canopy::...)`."""
    keys = [k for k in body if k.endswith('canopy')]
    if not keys:
        return 'None'
    key, c = keys[0], body[keys[0]]
    leaf = lambda: block(c['leaf_block'])  # noqa: E731
    if key == 'canopy':
        slope = c.get('canopy_slope', {})
        variation = c.get('variation_chance', [])
        variation = variation if isinstance(variation, list) else [variation]
        deco = c.get('canopy_decoration')
        deco = ('None' if deco is None else
                f'Some(CanopyDecoration {{ block: {decorative(deco["decoration_block"])}, chance: {chance(deco.get("decoration_chance"))}, '
                f'steps: {rng(deco.get("num_steps", 0))} }})')
        rust = (f'Simple(SimpleCanopy {{ leaf: {leaf()}, offset: ({int(c["canopy_offset"]["min"])}, {int(c["canopy_offset"]["max"])}), '
                f'min_width: {int(c.get("min_width", 1))}, rise: {int(slope.get("rise", 2))}, run: {int(slope.get("run", 1))}, '
                f'variation_chance: &[{", ".join(chance(v) for v in variation)}], decoration: {deco} }})')
    elif key == 'acacia_canopy':
        rust = f'Acacia {{ leaf: {leaf()}, size: {int(c["canopy_size"])}, simplify: {str(c.get("simplify_canopy", False)).lower()} }}'
    elif key == 'pine_canopy':
        rust = f'Pine {{ leaf: {leaf()}, height: {rng(c["canopy_height"])}, base_radius: {int(c["base_radius"])} }}'
    elif key == 'spruce_canopy':
        rust = (f'Spruce {{ leaf: {leaf()}, lower_offset: {rng(c["lower_offset"])}, upper_offset: {rng(c["upper_offset"])}, '
                f'max_radius: {rng(c["max_radius"])} }}')
    elif key == 'fancy_canopy':
        rust = f'Fancy {{ leaf: {leaf()}, height: {int(c["height"])}, radius: {int(c["radius"])} }}'
    elif key == 'mega_canopy':
        rust = (f'Mega {{ leaf: {leaf()}, height: {rng(c["canopy_height"])}, base_radius: {int(c.get("base_radius", 2))}, '
                f'core_width: {int(c.get("core_width", 1))}, simplify: {str(c.get("simplify_canopy", False)).lower()} }}')
    elif key == 'mega_pine_canopy':
        rust = (f'MegaPine {{ leaf: {leaf()}, height: {rng(c["canopy_height"])}, base_radius: {int(c.get("base_radius", 2))}, '
                f'radius_step_modifier: {float(c.get("radius_step_modifier", 3.5))!r}, core_width: {int(c.get("core_width", 1))} }}')
    elif key == 'roofed_canopy':
        rust = (f'Roofed {{ leaf: {leaf()}, height: {int(c["canopy_height"])}, core_width: {int(c["core_width"])}, '
                f'outer_radius: {int(c["outer_radius"])}, inner_radius: {int(c["inner_radius"])} }}')
    elif key == 'mangrove_canopy':
        rust = (f'Mangrove(MangroveCanopy {{ height: {rng(c["canopy_height"])}, radius: {rng(c["canopy_radius"])}, '
                f'attempts: {int(c["leaf_placement_attempts"])}, leaves: {weighted_blocks(c["leaf_blocks"])}, '
                f'hanging: {block(c["hanging_block"])}, hanging_chance: {chance(c.get("hanging_block_placement_chance"))}, '
                f'decoration: {decoration(c.get("canopy_decoration"))} }})')
    elif key == 'cherry_canopy':
        rust = (f'Cherry(CherryCanopy {{ leaf: {leaf()}, height: {rng(c["height"])}, radius: {rng(c["radius"])}, '
                f'trunk_width: {int(c.get("trunk_width", 1))}, wide_bottom_hole: {chance(c["wide_bottom_layer_hole_chance"])}, '
                f'corner_hole: {chance(c["corner_hole_chance"])}, hanging_leaves: {chance(c["hanging_leaves_chance"])}, '
                f'hanging_leaves_extension: {chance(c["hanging_leaves_extension_chance"])} }})')
    elif key == 'random_spread_canopy':
        rust = (f'RandomSpread {{ leaves: {weighted_blocks(c["leaf_blocks"])}, height: {int(c["canopy_height"])}, '
                f'radius: {int(c["canopy_radius"])}, attempts: {int(c["leaf_placement_attempts"])} }}')
    elif key == 'poplar_canopy':
        radius = ', '.join(f'({int(r["value"])}, {int(r["weight"])})' for r in c['radius'])
        rust = (f'Poplar(PoplarCanopy {{ leaf: {leaf()}, branch_x: {axis(c["branch_block"], "x")}, branch_z: {axis(c["branch_block"], "z")}, '
                f'radius: &[{radius}], height: {rng(c["height"])}, side_hole_chance: {float(c["side_hole_chance"])!r} }})')
    else:
        raise NotPorted(key)
    return f'Some(&Canopy::{rust})'


def intervals(height):
    return '&[' + ', '.join(str(int(i)) for i in height.get('intervals', [])) + ']'


def trunk(body, index):
    if 'trunk' in body:
        t = body['trunk']
        sub = t.get('can_be_submerged', False)
        depth = 255 if sub is True else int(sub.get('max_depth', 0)) if isinstance(sub, dict) else 0
        return (f'Simple(SimpleTrunk {{ block: {block(t["trunk_block"])}, height: {rng(t["trunk_height"])}, '
                f'height_modifier: {rng(t.get("height_modifier"), 0)}, max_depth: {depth}, decoration: {decoration(t.get("trunk_decoration"))} }})')
    if 'acacia_trunk' in body:
        t = body['acacia_trunk']
        lean, b, h = t['trunk_lean'], t.get('branches'), t['trunk_height']
        branches = ('AcaciaBranches { length: Range(1, 5), position: Range(1, 3), chance: Odds::Percent(0.0), canopy: None }' if b is None else
                    f'AcaciaBranches {{ length: {rng(b.get("branch_length"), [1, 5])}, position: {rng(b.get("branch_position"), [1, 3])}, '
                    f'chance: {chance(b.get("branch_chance"))}, canopy: {canopy(b.get("branch_canopy", {}))} }}')
        return (f'Acacia(AcaciaTrunk {{ block: {block(t["trunk_block"])}, width: {int(t["trunk_width"])}, height_base: {int(h["base"])}, '
                f'height_intervals: {intervals(h)}, min_height_for_canopy: {int(h.get("min_height_for_canopy", 3))}, '
                f'lean_height: {rng(lean["lean_height"])}, lean_steps: {rng(lean["lean_steps"])}, lean_length: {rng(lean.get("lean_length"), 0)}, '
                f'allow_diagonal: {str(lean.get("allow_diagonal_growth", True)).lower()}, decoration: {decoration(t.get("trunk_decoration"))}, '
                f'branches: {branches} }})')
    if 'fancy_trunk' in body:
        t = body['fancy_trunk']
        h, b = t['trunk_height'], t['branches']
        return (f'Fancy(FancyTrunk {{ block: {block(t["trunk_block"])}, width: {int(t["trunk_width"])}, height_base: {int(h["base"])}, '
                f'height_variance: {int(h["variance"])}, height_scale: {float(h["scale"])!r}, branch_slope: {float(b["slope"])!r}, '
                f'branch_density: {float(b["density"])!r}, min_altitude_factor: {float(b["min_altitude_factor"])!r}, '
                f'foliage_altitude_factor: {float(t["foliage_altitude_factor"])!r}, width_scale: {float(t["width_scale"])!r} }})')
    if 'mega_trunk' in body:
        t = body['mega_trunk']
        h, b = t['trunk_height'], t.get('branches')
        branches = 'None' if b is None else (
            f'Some(MegaBranches {{ length: {int(b["branch_length"])}, slope: {float(b["branch_slope"])!r}, interval: {rng(b["branch_interval"])}, '
            f'altitude_factor: ({float(b["branch_altitude_factor"]["min"])!r}, {float(b["branch_altitude_factor"]["max"])!r}), '
            f'canopy: {canopy(b.get("branch_canopy", {}))} }})')
        return (f'Mega(MegaTrunk {{ block: {block(t["trunk_block"])}, width: {int(t["trunk_width"])}, height_base: {int(h["base"])}, '
                f'height_intervals: {intervals(h)}, decoration: {decoration(t.get("trunk_decoration"))}, branches: {branches} }})')
    if 'mangrove_trunk' in body:
        t = body['mangrove_trunk']
        h, b = t['trunk_height'], t.get('branches', {})
        return (f'Mangrove(MangroveTrunk {{ block: {block(t["trunk_block"])}, height_base: {int(h["base"])}, '
                f'height_rand_a: {int(h["height_rand_a"])}, height_rand_b: {int(h["height_rand_b"])}, '
                f'branch_length: {rng(b.get("branch_length"), 0)}, branch_steps: {rng(b.get("branch_steps"), 0)}, '
                f'decoration: {decoration(t.get("trunk_decoration"))} }})')
    if 'cherry_trunk' in body:
        t = body['cherry_trunk']
        h, b = t['trunk_height'], t['branches']
        w = b['tree_type_weights']
        return (f'Cherry(CherryTrunk {{ block: {block(t["trunk_block"])}, block_x: {axis(t["trunk_block"], "x")}, '
                f'block_z: {axis(t["trunk_block"], "z")}, height_base: {int(h["base"])}, height_intervals: {intervals(h)}, '
                f'weights: [{int(w["one_branch"])}, {int(w["two_branches"])}, {int(w["two_branches_and_trunk"])}], '
                f'horizontal_length: {rng(b["branch_horizontal_length"])}, start_offset: {rng(b["branch_start_offset_from_top"])}, '
                f'end_offset: {rng(b["branch_end_offset_from_top"])} }})')
    if 'fallen_trunk' in body:
        t = body['fallen_trunk']
        feature = t.get('log_decoration_feature')
        feature = 'None' if feature is None else f'Some({index(feature)})'
        return (f'Fallen(FallenTrunk {{ block: {block(t["trunk_block"])}, block_x: {axis(t["trunk_block"], "x")}, '
                f'block_z: {axis(t["trunk_block"], "z")}, log_length: {rng(t["log_length"])}, '
                f'height_modifier: {rng(t.get("height_modifier"), 0)}, stump_height: {rng(t.get("stump_height"), 1)}, '
                f'log_decoration: {feature}, decoration: {decoration(t.get("trunk_decoration"))} }})')
    if 'poplar_trunk' in body:
        t = body['poplar_trunk']
        feature = t.get('log_decoration_feature')
        feature = 'None' if feature is None else f'Some({index(feature)})'
        return (f'Poplar(PoplarTrunk {{ block: {block(t["trunk_block"])}, block_x: {axis(t["trunk_block"], "x")}, '
                f'block_z: {axis(t["trunk_block"], "z")}, height: {rng(t["trunk_height"])}, '
                f'above_branches: {rng(t["remaining_trunk_height_above_branches"])}, '
                f'branches: {rng(t["amount_of_foliage_support_branches"])}, log_decoration: {feature} }})')
    raise NotPorted('tree trunk ' + ', '.join(body))


def roots(body):
    r = body.get('mangrove_roots')
    if r is None:
        return 'None'
    above = r.get('above_root')
    above = 'None' if above is None else f'Some(({block(above["above_root_block"])}, {chance(above.get("above_root_chance"))}))'
    return (f'Some(&MangroveRoots {{ max_width: {int(r["max_root_width"])}, max_length: {int(r["max_root_length"])}, '
            f'y_offset: {rng(r["y_offset"])}, root: {block(r["root_block"])}, muddy_root: {block(r["muddy_root_block"])}, '
            f'mud: {block(r["mud_block"])}, above_root: {above}, may_grow_through: {block_set(r["roots_may_grow_through"])}, '
            f'decoration: {decoration(r.get("root_decoration"))} }})')


def tree(body, index):
    """`index` maps a feature name to its `FEATURES` index (fallen trunks' log decorations)."""
    cluster = body.get('base_cluster')
    cluster = 'None' if cluster is None else (
        f'Some(BaseCluster {{ may_replace: {block_set(cluster["may_replace"])}, count: {int(cluster["num_clusters"])}, '
        f'radius: {int(cluster["cluster_radius"])} }})')
    base = body.get('base_block', [])
    return (f'Feature::Tree(Tree {{ trunk: Trunk::{trunk(body, index)}, canopy: {canopy(body)}, '
            f'may_grow_on: {block_set(body.get("may_grow_on", []))}, may_replace: {block_set(body.get("may_replace", []))}, '
            f'may_grow_through: {block_set(body.get("may_grow_through", []))}, '
            f'base_block: &[{", ".join(block(b) for b in (base if isinstance(base, list) else [base]))}], '
            f'roots: {roots(body)}, base_cluster: {cluster} }})')
