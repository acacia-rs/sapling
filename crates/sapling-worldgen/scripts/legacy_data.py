"""The legacy tree, foliage and rock features BDS builds in code (src/feature/legacy_tree.rs): per name,
a function of the feature indexer giving the Rust `Feature`. Counts, pickers and feature lists are
`LegacyTreeFeature`'s constructor (149e981a0) per type; the leaf litter variants of forests are the
ones BDS 1.26.52 picks for new worlds."""

GRASS = 'minecraft:scatter_tall_grass_around_tree_feature'
# name: (count, picker, features)
TREES = {
    'bamboo_jungle': (30.0, 'BambooJungle', ['fancy_oak_tree_feature', 'jungle_bush_feature', 'mega_jungle_tree_feature',
                                             'scatter_tall_grass_around_tree_feature']),
    'birch_forest': (10.0, 'First', ['select_birch_tree_feature', 'birch_tree_with_beehive_feature']),
    'birch_forest_mutated': (10.0, 'BirchMutated', ['select_birch_tree_feature', 'select_super_birch_tree_feature']),
    'extreme_hills_plus_trees': (3.0, 'ExtremeHills', ['select_spruce_tree_feature', 'fancy_oak_tree_feature', 'select_oak_tree_feature']),
    'flower_forest': (6.0, 'Forest', ['select_birch_tree_feature', 'fancy_oak_tree_with_optional_beehive_feature',
                                      'oak_tree_with_optional_beehive_feature']),
    'meadow_trees': (0.01, 'Meadow', ['fancy_oak_tree_with_beehive_feature', 'super_birch_tree_with_beehive_feature']),
    'forest': (10.0, 'Forest', ['select_birch_tree_with_leaf_litter_feature', 'fancy_oak_tree_with_leaf_litter_and_optional_beehive',
                                'select_oak_tree_with_leaf_litter_feature']),
    'ice': (0.1, 'First', ['select_spruce_tree_feature']),
    'jungle': (50.0, 'Jungle', ['fancy_oak_tree_feature', 'jungle_bush_feature', 'mega_jungle_tree_feature', 'select_jungle_tree_feature']),
    'jungle_edge': (2.0, 'JungleEdge', ['fancy_oak_tree_with_optional_beehive_feature', 'jungle_bush_feature', 'select_jungle_tree_feature']),
    'mesa_plateau_stone': (5.0, 'First', ['select_oak_tree_with_leaf_litter_feature']),
    'plains': (0.05, 'OneInThree', ['fancy_oak_tree_with_optional_beehive_feature', 'select_oak_tree_feature',
                                    'oak_tree_with_beehive_feature']),
    'savanna': (1.0, 'FourInFive', ['savanna_tree_feature', 'select_oak_tree_feature']),
    'savanna_mutated': (2.0, 'FourInFive', ['savanna_tree_feature', 'select_oak_tree_feature']),
    'taiga': (10.0, 'OneInThree', ['pine_tree_feature', 'select_spruce_tree_feature']),
    'mega_taiga': (10.0, 'MegaTaiga', ['mega_spruce_tree_feature', 'mega_pine_tree_feature', 'pine_tree_feature',
                                       'select_spruce_tree_feature']),
    'redwood_taiga_mutated': (10.0, 'RedwoodMutated', ['mega_spruce_tree_feature', 'pine_tree_feature', 'select_spruce_tree_feature']),
}


def _trees(count, picker, features):
    def rust(index):
        listed = ', '.join(str(index('minecraft:' + f)) for f in features)
        return (f'Feature::LegacyTree(LegacyTree::Trees {{ count: {count!r}, choice: TreeChoice::{picker}, features: &[{listed}], '
                f'grass: {index(GRASS)} }})')
    return rust


def _tree_name(key):
    return f'minecraft:legacy:{key}_feature' if key == 'meadow_trees' else f'minecraft:legacy:{key}_tree_feature'


LEGACY = {_tree_name(key): _trees(*value) for key, value in TREES.items()}
LEGACY.update({
    'minecraft:legacy:forest_foliage_feature': lambda index: 'Feature::LegacyTree(LegacyTree::ForestFoliage { flower_forest: false })',
    'minecraft:legacy:flower_forest_foliage_feature': lambda index: 'Feature::LegacyTree(LegacyTree::ForestFoliage { flower_forest: true })',
    'minecraft:legacy:swamp_foliage_feature': lambda index: (
        f'Feature::LegacyTree(LegacyTree::SwampFoliage {{ seagrass: {index("minecraft:seagrass_feature")}, '
        f'tree: {index("minecraft:swamp_tree_feature")}, huge_mushroom: {index("minecraft:huge_mushroom_feature")}, '
        f'grass: {index(GRASS)} }})'),
    'minecraft:legacy:forest_rock_feature': lambda index: 'Feature::LegacyTree(LegacyTree::ForestRock)',
    'minecraft:leaf_litter_feature': lambda index: 'Feature::LegacyTree(LegacyTree::LeafLitter)',
    'minecraft:huge_mushroom_feature': lambda index: 'Feature::LegacyTree(LegacyTree::HugeMushroom)',
})
