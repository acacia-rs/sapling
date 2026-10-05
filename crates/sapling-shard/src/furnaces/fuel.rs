//! What burns in a furnace and for how long. BDS has this compiled in; the numbers are the
//! game's as its wiki lists them (how many items one piece of fuel smelts), and the wooden
//! kinds are told by the item tags of sapling-data (docs/research/containers-crafting.md, D).
// TODO: wooden stairs, fences, doors, signs, banners, bows and the like do not burn here.

/// The fuel that leaves its bucket behind.
pub(super) const LAVA_BUCKET: &str = "minecraft:lava_bucket";
pub(super) const BUCKET: &str = "minecraft:bucket";

/// Fuels by name, and by the end of their name: each with the items one of it smelts.
const NAMED: &[(&str, f32)] = &[
    (LAVA_BUCKET, 100.0),
    ("minecraft:coal_block", 80.0),
    ("minecraft:dried_kelp_block", 20.0),
    ("minecraft:blaze_rod", 12.0),
    ("minecraft:stick", 0.5),
    ("minecraft:bowl", 0.5),
    ("minecraft:bamboo", 0.25),
    ("minecraft:scaffolding", 0.25),
    ("minecraft:crafting_table", 1.5),
    ("minecraft:bookshelf", 1.5),
    ("minecraft:chest", 1.5),
    ("minecraft:barrel", 1.5),
];
const ENDING: &[(&str, f32)] = &[("_boat", 6.0), ("_sapling", 0.5), ("_carpet", 0.335)];
/// Wooden tools, which carry no tag of their own here.
const WOODEN: (&str, f32) = ("minecraft:wooden_", 1.0);
const TAGGED: &[(&str, f32)] = &[
    ("minecraft:coals", 8.0),
    ("minecraft:logs_that_burn", 1.5),
    ("minecraft:planks", 1.5),
    ("minecraft:wooden_slabs", 0.75),
    ("minecraft:wool", 0.5),
];

fn smelts(name: &str) -> Option<f32> {
    let named = NAMED.iter().find(|(fuel, _)| *fuel == name);
    // The nether's woods are planks and slabs like the others, and do not burn.
    let is_fireproof = name.contains("crimson") || name.contains("warped");
    let tagged = || TAGGED.iter().find(|(tag, _)| !is_fireproof && sapling_data::has_tag(name, tag));
    let ending = || ENDING.iter().find(|(end, _)| name.ends_with(end));
    let wooden = || Some(&WOODEN).filter(|(start, _)| name.starts_with(start) && !name.ends_with("_door"));
    named.or_else(tagged).or_else(ending).or_else(wooden).map(|(_, items)| *items)
}

/// Ticks one piece of the fuel `name` burns in a smelter that takes `interval` ticks an item;
/// `None` for what does not burn.
pub(super) fn burn_ticks(name: &str, interval: u16) -> Option<u16> {
    smelts(name).map(|items| (f32::from(interval) * items).round() as u16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuels_burn_as_long_as_the_items_they_smelt_take() {
        let ticks = |name: &str| burn_ticks(&format!("minecraft:{name}"), 200);
        assert_eq!([ticks("coal"), ticks("charcoal"), ticks("oak_planks"), ticks("birch_log"), ticks("stick"), ticks("lava_bucket")], [Some(1600), Some(1600), Some(300), Some(300), Some(100), Some(20000)]);
        assert_eq!([ticks("wooden_pickaxe"), ticks("oak_boat"), ticks("white_wool"), ticks("oak_slab")], [Some(200), Some(1200), Some(100), Some(150)]);
        assert_eq!([ticks("stone"), ticks("crimson_planks"), ticks("iron_ingot")], [None, None, None]);
        assert_eq!(burn_ticks("minecraft:coal", 100), Some(800), "a blast furnace's coal smelts as many, twice as fast");
    }
}
