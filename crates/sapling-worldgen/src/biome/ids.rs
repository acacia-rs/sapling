//! cubiomes' biome ids (Java's legacy numbering plus 1.18+ additions) to Bedrock biome ids.

/// Bedrock's id for a cubiomes id; None for biomes the overworld tree never yields.
pub fn bedrock_id(cubiomes: u8) -> Option<u32> {
    Some(match cubiomes {
        0 => 0,      // ocean
        1 => 1,      // plains
        2 => 2,      // desert
        3 => 3,      // windswept_hills (extreme_hills)
        4 => 4,      // forest
        5 => 5,      // taiga
        6 => 6,      // swamp (swampland)
        7 => 7,      // river
        10 => 46,    // frozen_ocean
        11 => 11,    // frozen_river
        12 => 12,    // snowy_plains (ice_plains)
        14 => 14,    // mushroom_fields (mushroom_island)
        16 => 16,    // beach
        21 => 21,    // jungle
        23 => 23,    // sparse_jungle (jungle_edge)
        24 => 24,    // deep_ocean
        25 => 25,    // stony_shore (stone_beach)
        26 => 26,    // snowy_beach (cold_beach)
        27 => 27,    // birch_forest
        29 => 29,    // dark_forest (roofed_forest)
        30 => 30,    // snowy_taiga (cold_taiga)
        32 => 32,    // old_growth_pine_taiga (mega_taiga)
        34 => 34,    // windswept_forest (extreme_hills_plus_trees)
        35 => 35,    // savanna
        36 => 36,    // savanna_plateau
        37 => 37,    // badlands (mesa)
        38 => 38,    // wooded_badlands (mesa_plateau_stone)
        44 => 40,    // warm_ocean
        45 => 42,    // lukewarm_ocean
        46 => 44,    // cold_ocean
        47 => 41,    // deep_warm_ocean
        48 => 43,    // deep_lukewarm_ocean
        49 => 45,    // deep_cold_ocean
        50 => 47,    // deep_frozen_ocean
        129 => 129,  // sunflower_plains
        131 => 131,  // windswept_gravelly_hills (extreme_hills_mutated)
        132 => 132,  // flower_forest
        140 => 140,  // ice_spikes (ice_plains_spikes)
        155 => 155,  // old_growth_birch_forest (birch_forest_mutated)
        160 => 160,  // old_growth_spruce_taiga (redwood_taiga_mutated)
        163 => 163,  // windswept_savanna (savanna_mutated)
        165 => 165,  // eroded_badlands (mesa_bryce)
        168 => 48,   // bamboo_jungle
        174 => 188,  // dripstone_caves
        175 => 187,  // lush_caves
        177 => 186,  // meadow
        178 => 185,  // grove
        179 => 184,  // snowy_slopes
        180 => 182,  // jagged_peaks
        181 => 183,  // frozen_peaks
        182 => 189,  // stony_peaks
        183 => 190,  // deep_dark
        184 => 191,  // mangrove_swamp
        185 => 192,  // cherry_grove
        186 => 193,  // pale_garden
        187 => 194,  // sulfur_caves
        _ => return None,
    })
}
