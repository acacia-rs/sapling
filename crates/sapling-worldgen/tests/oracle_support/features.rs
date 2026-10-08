//! Feature blocks per family: precision over the cells our features changed, recall over the
//! cells where BDS has a family block the terrain before features does not explain.

use std::collections::BTreeMap;

use sapling_worldgen::Block;

use super::BdsCell;

/// The family of a block a feature places: ores without their deepslate variant, stone blobs,
/// logs and leaves per species.
pub(super) fn family(name: &str) -> Option<&'static str> {
    if let Some(wood) = WOOD.iter().find(|(block, _)| *block == name) {
        return Some(wood.1);
    }
    let name = name.strip_prefix("deepslate_").unwrap_or(name);
    Some(match name {
        "andesite" | "diorite" | "granite" => "stone blobs",
        "tuff" => "tuff",
        "dirt" => "dirt",
        "gravel" => "gravel",
        "clay" => "clay",
        "sand" => "sand",
        "coal_ore" => "coal",
        "iron_ore" => "iron",
        "copper_ore" => "copper",
        "gold_ore" => "gold",
        "redstone_ore" | "lit_redstone_ore" => "redstone",
        "lapis_ore" => "lapis",
        "diamond_ore" => "diamond",
        "emerald_ore" => "emerald",
        "infested_stone" | "infested_deepslate" => "infested",
        "short_grass" | "fern" | "tall_grass" | "large_fern" => "grass, ferns",
        "short_dry_grass" | "tall_dry_grass" => "dry grass",
        "bush" => "bushes",
        "sunflower" => "sunflowers",
        "seagrass" => "seagrass",
        "kelp" => "kelp",
        "amethyst_block" | "budding_amethyst" | "smooth_basalt" | "calcite" => "geode shells",
        "amethyst_cluster" | "large_amethyst_bud" | "medium_amethyst_bud" | "small_amethyst_bud" => "amethyst buds",
        "deadbush" => "dead bushes",
        "reeds" => "sugar cane",
        "cactus" => "cacti",
        "brown_mushroom" | "red_mushroom" => "mushrooms",
        "waterlily" => "lily pads",
        "pumpkin" | "melon_block" => "pumpkins, melons",
        "sweet_berry_bush" => "sweet berries",
        "poppy" | "dandelion" | "blue_orchid" | "azure_bluet" | "red_tulip" | "orange_tulip" | "white_tulip" | "pink_tulip" | "oxeye_daisy"
        | "cornflower" | "lily_of_the_valley" | "allium" => "flowers",
        "lilac" | "rose_bush" | "peony" => "tall flowers",
        "vine" => "vines",
        "cocoa" => "cocoa",
        "bee_nest" => "bee nests",
        "podzol" => "podzol",
        "bamboo" => "bamboo",
        "pale_moss_block" => "pale moss",
        "pale_moss_carpet" => "pale moss carpets",
        "pale_hanging_moss" => "pale hanging moss",
        "closed_eyeblossom" | "open_eyeblossom" => "eyeblossoms",
        "creaking_heart" => "creaking hearts",
        "mossy_cobblestone" => "forest rocks",
        "brown_mushroom_block" | "red_mushroom_block" | "mushroom_stem" => "huge mushrooms",
        "mangrove_roots" | "muddy_mangrove_roots" => "mangrove roots",
        "moss_carpet" => "moss carpets",
        "mangrove_propagule" => "propagules",
        "leaf_litter" => "leaf litter",
        "pink_petals" => "pink petals",
        "wildflowers" => "wildflowers",
        "moss_block" => "moss",
        "cave_vines" | "cave_vines_body_with_berries" | "cave_vines_head_with_berries" => "cave vines",
        "big_dripleaf" | "small_dripleaf_block" => "dripleaves",
        "azalea" | "flowering_azalea" => "azaleas",
        "spore_blossom" => "spore blossoms",
        "dirt_with_roots" | "hanging_roots" => "azalea roots",
        "pointed_dripstone" | "dripstone_block" => "dripstone",
        "sulfur_spike" => "sulfur spikes",
        "glow_lichen" => "glow lichen",
        "magma" => "magma",
        "bubble_column" => "bubble columns",
        "packed_ice" => "packed ice",
        "cobblestone" | "mob_spawner" | "chest" => "monster rooms",
        "sea_pickle" => "sea pickles",
        n if n.ends_with("coral_block") => "coral blocks",
        n if n.ends_with("coral_fan") || n.ends_with("coral_wall_fan") => "coral fans",
        n if n.ends_with("_coral") => "coral plants",
        _ => return None,
    })
}

const WOOD: &[(&str, &str)] = &[
    ("oak_log", "oak logs"),
    ("spruce_log", "spruce logs"),
    ("birch_log", "birch logs"),
    ("jungle_log", "jungle logs"),
    ("acacia_log", "acacia logs"),
    ("dark_oak_log", "dark oak logs"),
    ("mangrove_log", "mangrove logs"),
    ("cherry_log", "cherry logs"),
    ("pale_oak_log", "pale oak logs"),
    ("oak_leaves", "oak leaves"),
    ("spruce_leaves", "spruce leaves"),
    ("birch_leaves", "birch leaves"),
    ("jungle_leaves", "jungle leaves"),
    ("acacia_leaves", "acacia leaves"),
    ("dark_oak_leaves", "dark oak leaves"),
    ("mangrove_leaves", "mangrove leaves"),
    ("cherry_leaves", "cherry leaves"),
    ("pale_oak_leaves", "pale oak leaves"),
    ("azalea_leaves", "azalea leaves"),
    ("azalea_leaves_flowered", "azalea leaves"),
];

#[derive(Default)]
struct Counts {
    placed: u64,
    wanted: u64,
    hits: u64,
}

#[derive(Default)]
pub struct FeatureTally(BTreeMap<&'static str, Counts>);

impl FeatureTally {
    /// One stable cell: our block before and after features, and BDS's.
    /// Cells BDS gave a block no terrain stage or ported feature places (sulfur caves, structures) are skipped.
    pub fn add(&mut self, before: Block, ours: Block, bds: &BdsCell) {
        if Block::from_name(&bds.block).is_none() && !bds.is_empty() && !bds.is_fluid() {
            return;
        }
        let featured = before != ours;
        let hit = featured && bds.is(ours);
        if featured && let Some(family) = family(ours.name()) {
            let counts = self.0.entry(family).or_default();
            counts.placed += 1;
            counts.hits += u64::from(hit);
        }
        if bds.block != before.name() && let Some(family) = family(&bds.block) {
            self.0.entry(family).or_default().wanted += 1;
        }
    }

    pub fn print(&self) {
        println!("  features: family, precision (hits/placed), recall (hits/wanted)");
        for (family, c) in &self.0 {
            let percent = |n: u64, of: u64| 100.0 * n as f64 / of.max(1) as f64;
            println!(
                "    {family:>12}: precision {:7.3}% ({}/{}), recall {:7.3}% ({}/{})",
                percent(c.hits, c.placed),
                c.hits,
                c.placed,
                percent(c.hits, c.wanted),
                c.hits,
                c.wanted
            );
        }
    }
}
