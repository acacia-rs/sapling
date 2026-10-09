//! What burns in a furnace and for how long, by BDS's `FurnaceBlockActor::getBurnDuration` and
//! `getItemBurnDuration` (docs/research/furnaces.md, section 1): a factor, the items one piece
//! smelts, decided by the block an item places when it places one, else by the item. Blocks of
//! wood the game names by material; here they are told by name.

/// The fuel that leaves its bucket behind.
pub(super) const LAVA_BUCKET: &str = "minecraft:lava_bucket";
pub(super) const BUCKET: &str = "minecraft:bucket";

/// Items with a fuel component of their own: its duration over 10.
const COMPONENT: &[(&str, f32)] = &[("coal", 8.0), ("charcoal", 8.0), ("wooden_spear", 1.0)];
/// Blocks that are wood but do not burn (BDS's list of 48 names).
const FIREPROOF: &[&str] = &[
    "crimson",
    "warped",
    "chorus_",
    "mushroom_block",
    "mushroom_stem",
    "creaking_heart",
    "beehive",
    "bee_nest",
];
const SPECIES: &[&str] = &[
    "oak", "spruce", "birch", "jungle", "acacia", "dark_oak", "mangrove", "cherry", "pale_oak",
    "bamboo", "poplar",
];
/// What a wooden species' blocks are called after its name.
const WOODEN_PARTS: &[&str] = &[
    "_stairs",
    "_fence",
    "_fence_gate",
    "_trapdoor",
    "_pressure_plate",
    "_button",
    "_shelf",
    "_mosaic",
    "_mosaic_slab",
    "_mosaic_stairs",
    "_block",
];
/// Blocks of wood with names of their own.
const WOODEN: &[&str] = &[
    "fence_gate",
    "trapdoor",
    "wooden_button",
    "wooden_pressure_plate",
    "chest",
    "trapped_chest",
    "bookshelf",
    "chiseled_bookshelf",
    "crafting_table",
    "cartography_table",
    "fletching_table",
    "smithing_table",
    "loom",
    "barrel",
    "lectern",
    "composter",
    "jukebox",
    "noteblock",
    "daylight_detector",
    "daylight_detector_inverted",
    "ladder",
];
const BUTTONS: &[&str] = &["wooden_button", "_button"];
const SMALL_PLANTS: &[&str] = &[
    "_sapling",
    "mangrove_propagule",
    "azalea",
    "flowering_azalea",
    "deadbush",
    "leaf_litter",
    "short_dry_grass",
    "tall_dry_grass",
];
const BLOCKS: &[(&str, f32)] = &[
    ("coal_block", 80.0),
    ("dried_kelp_block", 20.0),
    ("bamboo", 0.25),
    ("scaffolding", 0.25),
];
/// Items' own factors (BDS item registration).
const ITEMS: &[(&str, f32)] = &[
    ("lava_bucket", 100.0),
    ("blaze_rod", 12.0),
    ("fishing_rod", 1.5),
    ("banner", 1.5),
    ("mangrove_roots", 1.5),
    ("bow", 1.0),
    ("crossbow", 1.0),
    ("bowl", 1.0),
    ("wooden_door", 1.0),
    ("stick", 0.5),
    ("short_dry_grass", 0.5),
    ("tall_dry_grass", 0.5),
];
/// Items' own factors by the end of their name, for the burning species.
const ITEM_ENDINGS: &[(&str, f32)] = &[
    ("_chest_boat", 6.0),
    ("_boat", 6.0),
    ("_raft", 6.0),
    ("_chest_raft", 6.0),
    ("_hanging_sign", 4.0),
    ("_door", 1.0),
    ("_sign", 1.0),
    ("_cushion", 0.25),
];

fn is_fireproof(name: &str) -> bool {
    FIREPROOF.iter().any(|part| name.contains(part))
}

/// Whether the block `name` (without namespace) is of wood.
fn is_wood(name: &str, namespaced: &str) -> bool {
    let tagged = [
        "minecraft:planks",
        "minecraft:logs_that_burn",
        "minecraft:wooden_slabs",
    ]
    .iter()
    .any(|tag| sapling_data::has_tag(namespaced, tag));
    let species_part = SPECIES.iter().any(|species| {
        name.strip_prefix(species)
            .is_some_and(|part| WOODEN_PARTS.contains(&part))
    });
    tagged || species_part || WOODEN.contains(&name)
}

/// The block rules (`getItemBurnDuration` b to h); `None` when they do not decide.
fn as_block(name: &str, namespaced: &str) -> Option<f32> {
    if is_fireproof(name) {
        return Some(0.0);
    }
    let is_wooden_button = BUTTONS.iter().any(|end| name.ends_with(end))
        && (name == "wooden_button" || SPECIES.iter().any(|species| name.starts_with(species)));
    if is_wooden_button
        || SMALL_PLANTS
            .iter()
            .any(|plant| name == *plant || plant.starts_with('_') && name.ends_with(plant))
    {
        return Some(0.5);
    }
    if is_wood(name, namespaced) {
        return Some(1.5);
    }
    BLOCKS
        .iter()
        .find(|(block, _)| *block == name)
        .map(|(_, items)| *items)
}

fn as_item(name: &str) -> Option<f32> {
    let named = ITEMS.iter().find(|(item, _)| *item == name);
    let ending = || {
        ITEM_ENDINGS.iter().find(|(end, _)| {
            name.ends_with(end) && SPECIES.iter().any(|species| name.starts_with(species))
        })
    };
    let tool = || {
        name.strip_prefix("wooden_")
            .filter(|tool| ["sword", "shovel", "pickaxe", "axe", "hoe"].contains(tool))
            .map(|_| &("", 1.0))
    };
    named.or_else(ending).or_else(tool).map(|(_, items)| *items)
}

/// Items one piece of `name` smelts; 0 for what does not burn.
fn smelts(namespaced: &str) -> f32 {
    let name = namespaced.strip_prefix("minecraft:").unwrap_or(namespaced);
    if let Some((_, items)) = COMPONENT.iter().find(|(item, _)| *item == name) {
        return *items;
    }
    // Door, sign and boat items place no block of their own name; their factor is the item's.
    let item_first = ["_door", "_sign", "_boat", "_raft"]
        .iter()
        .any(|end| name.ends_with(end));
    let block = || {
        if item_first {
            None
        } else {
            as_block(name, namespaced)
        }
    };
    block().or_else(|| as_item(name)).unwrap_or(0.0)
}

/// Ticks one piece of the fuel `name` burns in a smelter that takes `interval` ticks an item,
/// truncated as BDS stores it; `None` for what does not burn.
pub(super) fn burn_ticks(name: &str, interval: u16) -> Option<u16> {
    let ticks = (f32::from(interval) * smelts(name)) as u16;
    (ticks > 0).then_some(ticks.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuels_burn_as_long_as_bds_says() {
        let ticks = |name: &str| burn_ticks(&format!("minecraft:{name}"), 200);
        assert_eq!(
            [
                ticks("coal"),
                ticks("charcoal"),
                ticks("coal_block"),
                ticks("lava_bucket"),
                ticks("blaze_rod")
            ],
            [Some(1600), Some(1600), Some(16000), Some(20000), Some(2400)]
        );
        assert_eq!(
            [
                ticks("oak_planks"),
                ticks("birch_log"),
                ticks("oak_slab"),
                ticks("spruce_stairs"),
                ticks("fence_gate"),
                ticks("chest")
            ],
            [Some(300); 6]
        );
        assert_eq!(
            [
                ticks("wooden_pickaxe"),
                ticks("bow"),
                ticks("bowl"),
                ticks("spruce_door"),
                ticks("oak_sign")
            ],
            [Some(200); 5]
        );
        assert_eq!(
            [
                ticks("oak_boat"),
                ticks("oak_chest_boat"),
                ticks("bamboo_raft"),
                ticks("oak_hanging_sign")
            ],
            [Some(1200), Some(1200), Some(1200), Some(800)]
        );
        assert_eq!(
            [
                ticks("stick"),
                ticks("oak_sapling"),
                ticks("wooden_button"),
                ticks("birch_button"),
                ticks("bamboo"),
                ticks("scaffolding")
            ],
            [
                Some(100),
                Some(100),
                Some(100),
                Some(100),
                Some(50),
                Some(50)
            ]
        );
        let none = [
            "stone",
            "crimson_planks",
            "warped_door",
            "white_wool",
            "white_carpet",
            "iron_ingot",
            "stone_button",
            "crimson_hanging_sign",
        ];
        assert_eq!(none.map(ticks), [None; 8]);
        assert_eq!(
            burn_ticks("minecraft:coal", 100),
            Some(800),
            "a blast furnace's coal smelts as many, twice as fast"
        );
    }
}
