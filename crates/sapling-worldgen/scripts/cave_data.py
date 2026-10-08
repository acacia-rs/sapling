"""The lush caves half of features_data.py: snap_to_surface, vegetation_patch and growing_plant
documents as the Rust features of src/feature/cave_spec.rs, and the hardcoded lush features of
src/feature/lush.rs."""
from blocks_data import BLOCKS, NotPorted, block

PORTED = {'minecraft:snap_to_surface_feature', 'minecraft:vegetation_patch_feature', 'minecraft:growing_plant_feature'}
_BIG_DRIPLEAVES = ['south', 'west', 'north', 'east']
SURFACES = {'ceiling': 'Ceiling', 'floor': 'Floor', 'random_horizontal': 'RandomHorizontal', 'random_vertical': 'RandomVertical'}
LEGACY = {
    'minecraft:moss_vegetation_feature': lambda index: 'Feature::Lush(Lush::MossVegetation)',
    'minecraft:vines_single_face_feature': lambda index: 'Feature::Lush(Lush::VinesSingleFace)',
    'minecraft:dripleaf_feature': lambda index: (
        'Feature::Lush(Lush::Dripleaf { big: ['
        + ', '.join(str(index(f'minecraft:big_dripleaf_{d}_feature')) for d in _BIG_DRIPLEAVES) + '] })'),
    'minecraft:azalea_root_system_feature': lambda index: (
        f'Feature::Lush(Lush::AzaleaRootSystem {{ tree: {index("minecraft:azalea_tree_feature")} }})'),
}


def rng(value):
    """An `{range_min, range_max}` or a constant as a `Range`."""
    if isinstance(value, dict):
        return f'Range({int(value["range_min"])}, {int(value["range_max"])})'
    return f'Range({int(value)}, {int(value)})'


def every_state(values):
    """A replaceable list: block types match whatever their states (the first variant of each
    (name, states) pair: the waterlogged twins never stand in terrain)."""
    out = []
    for value in values:
        try:
            single = block(value)
        except NotPorted:
            continue
        name = next(n for (n, _), v in BLOCKS.items() if 'Block::' + v == single)
        stated = isinstance(value, dict) and value.get('states')
        out += [single] if stated else ['Block::' + v for (n, _), v in BLOCKS.items() if n == name]
    return '&[' + ', '.join(dict.fromkeys(out)) + ']'


def weighted(entries):
    return '&[' + ', '.join(f'({block(b)}, {float(w)!r})' for b, w in entries) + ']'


def rust(kind, body, index):
    if kind == 'minecraft:snap_to_surface_feature':
        flag = lambda key, default: str(bool(body.get(key, default))).lower()  # noqa: E731
        allowed = ', '.join(block(b) for b in body.get('allowed_surface_blocks', []))
        return (f'Feature::SnapToSurface(SnapToSurface {{ feature: {index(body["feature_to_snap"])}, '
                f'range: {int(body["vertical_search_range"])}, surface: Surface::{SURFACES[body.get("surface", "floor")]}, '
                f'allow_air: {flag("allow_air_placement", True)}, allow_water: {flag("allow_underwater_placement", False)}, '
                f'allow_solid: {flag("allow_non_air_placement", False)}, embed: {flag("embed_in_surface", False)}, '
                f'allowed: &[{allowed}] }})')
    if kind == 'minecraft:vegetation_patch_feature':
        num = lambda key: repr(float(body.get(key, 0.0)))  # noqa: E731
        return (f'Feature::VegetationPatch(VegetationPatch {{ replaceable: {every_state(body["replaceable_blocks"])}, '
                f'ground: {block(body["ground_block"])}, vegetation: {index(body["vegetation_feature"])}, '
                f'depth: {rng(body.get("depth", 1))}, extra_deep_chance: {num("extra_deep_block_chance")}, '
                f'vertical_range: {int(body.get("vertical_range", 1))}, vegetation_chance: {num("vegetation_chance")}, '
                f'radius: {rng(body["horizontal_radius"])}, extra_edge_chance: {num("extra_edge_column_chance")}, '
                f'waterlogged: {str(body.get("waterlogged", False)).lower()}, ceiling: {str(body.get("surface") == "ceiling").lower()} }})')
    if kind == 'minecraft:growing_plant_feature':
        heights = ', '.join(f'({rng(r)}, {float(w)!r})' for r, w in body['height_distribution'])
        return (f'Feature::GrowingPlant(GrowingPlant {{ heights: &[{heights}], up: {str(body["growth_direction"].upper() == "UP").lower()}, '
                f'age: {rng(body.get("age", 0))}, body: {weighted(body["body_blocks"])}, head: {weighted(body["head_blocks"])}, '
                f'allow_water: {str(body.get("allow_water", False)).lower()} }})')
    raise NotPorted(kind)
