//! Structure blocks per structure kind: precision over the cells our structure stage changed,
//! recall over the cells inside a structure's area where BDS has a block that only a structure
//! places, and the blocks BDS has there (so unported kinds still show what they need).

use std::collections::BTreeMap;

use sapling_worldgen::Block;
use sapling_worldgen::structure::{BoundingBox, StructureKind};

use super::features::family;
use super::{BdsCell, TERRAIN_BLOCKS};

/// Blocks only cave decoration, geodes, lava flow or trees place.
const DECORATION: &[&str] = &[
    "glow_lichen",
    "moss_block",
    "moss_carpet",
    "big_dripleaf",
    "small_dripleaf_block",
    "cave_vines",
    "cave_vines_body_with_berries",
    "cave_vines_head_with_berries",
    "vine",
    "amethyst_block",
    "budding_amethyst",
    "small_amethyst_bud",
    "medium_amethyst_bud",
    "large_amethyst_bud",
    "amethyst_cluster",
    "calcite",
    "smooth_basalt",
    "pointed_dripstone",
    "dripstone_block",
    "magma",
    "obsidian",
    "bubble_column",
    "dirt_with_roots",
    "hanging_roots",
    "spore_blossom",
    "azalea",
    "flowering_azalea",
    "pink_petals",
    "leaf_litter",
    "azalea_leaves",
    "azalea_leaves_flowered",
    "oak_leaves",
    "spruce_leaves",
    "birch_leaves",
    "jungle_leaves",
    "acacia_leaves",
    "dark_oak_leaves",
    "mangrove_leaves",
    "cherry_leaves",
    "pale_oak_leaves",
    "snow_layer",
    "sculk",
    "sculk_vein",
    "brown_mushroom_block",
    "red_mushroom_block",
    "mushroom_stem",
    "bee_nest",
    "wildflowers",
    "peony",
    "rose_bush",
    "lilac",
    "firefly_bush",
    "oak_log",
    "spruce_log",
    "birch_log",
    "jungle_log",
    "acacia_log",
    "mangrove_log",
    "cherry_log",
    "pale_oak_log",
    "mangrove_roots",
    "muddy_mangrove_roots",
    "mangrove_propagule",
    "cocoa",
    "pale_moss_block",
    "pale_moss_carpet",
    "pale_hanging_moss",
    "creaking_heart",
    "resin_clump",
];

/// Blocks monster rooms place too: structure blocks only inside a structure's area. Logs count as
/// tree blocks (trees are unported), though villages and mesa mineshafts use some.
const AMBIGUOUS: &[&str] = &["cobblestone", "mossy_cobblestone", "mob_spawner", "chest", "dark_oak_log"];

/// Name parts of feature-only block families: deep-dark sculk, coral reefs, sulfur caves.
const DECORATION_PARTS: &[&str] = &["sculk", "coral", "sea_pickle", "sulfur", "cinnabar"];

/// A block BDS saved that no terrain stage, ported feature or known decoration explains.
fn structure_block(bds: &BdsCell, in_area: bool) -> bool {
    let name = bds.block.as_str();
    !(bds.is_empty() || bds.is_fluid())
        && !DECORATION_PARTS.iter().any(|part| name.contains(part))
        && !TERRAIN_BLOCKS.contains(&name)
        && family(name).is_none()
        && !DECORATION.contains(&name)
        && (in_area || !AMBIGUOUS.contains(&name))
}

#[derive(Default)]
struct Counts {
    placed: u64,
    hits: u64,
    /// Placed cells that are not air, and those BDS has too.
    solid_placed: u64,
    solid_hits: u64,
    wanted: u64,
    /// Wanted cells where our block is BDS's.
    found: u64,
    blocks: BTreeMap<String, u64>,
}

#[derive(Default)]
pub struct StructureTally {
    kinds: BTreeMap<&'static str, Counts>,
    unattributed: BTreeMap<String, u64>,
    starts: BTreeMap<&'static str, Vec<(i32, i32)>>,
}

/// The kind whose area holds the cell; where areas overlap, the start nearest in units of its
/// kind's reach (rough areas are squares of reach chunks around the start).
pub fn attribute(areas: &[(StructureKind, (i32, i32), BoundingBox)], (x, y, z): (i32, i32, i32)) -> Option<StructureKind> {
    let nearness = |kind: &StructureKind, (sx, sz): (i32, i32)| {
        let d = (x.div_euclid(16) - sx).abs().max((z.div_euclid(16) - sz).abs());
        (d * 1000) / (kind.reach() + 1)
    };
    areas.iter().filter(|(_, _, area)| area.contains(x, y, z)).min_by_key(|(kind, start, _)| nearness(kind, *start)).map(|(kind, _, _)| *kind)
}

impl StructureTally {
    /// Remembers the starts behind a chunk's areas, for the summary.
    pub fn note_areas(&mut self, areas: &[(StructureKind, (i32, i32), BoundingBox)]) {
        for (kind, start, _) in areas {
            let starts = self.starts.entry(kind.name()).or_default();
            if !starts.contains(start) {
                starts.push(*start);
            }
        }
    }

    /// One stable cell: our block before and after the structure stage, BDS's, the ported kind
    /// whose area holds it (what placed it) and the kind of any structure there (what BDS wants).
    pub fn add(&mut self, before: Block, after: Block, bds: &BdsCell, placer: Option<StructureKind>, kind: Option<StructureKind>) {
        if before != after {
            let counts = self.kinds.entry(placer.map_or("unattributed", |k| k.name())).or_default();
            counts.placed += 1;
            counts.hits += u64::from(bds.is(after));
            if after != Block::Air {
                counts.solid_placed += 1;
                counts.solid_hits += u64::from(bds.is(after));
            }
        }
        if bds.is(before) || !structure_block(bds, kind.is_some()) {
            return;
        }
        match kind {
            Some(kind) => {
                let counts = self.kinds.entry(kind.name()).or_default();
                counts.wanted += 1;
                counts.found += u64::from(bds.is(after));
                *counts.blocks.entry(bds.block.clone()).or_default() += 1;
            }
            None => *self.unattributed.entry(bds.block.clone()).or_default() += 1,
        }
    }

    pub fn print(&self) {
        let percent = |n: u64, of: u64| 100.0 * n as f64 / of.max(1) as f64;
        let top = |blocks: &BTreeMap<String, u64>| {
            let mut v: Vec<_> = blocks.iter().collect();
            v.sort_by(|a, b| b.1.cmp(a.1));
            v.iter().take(5).map(|(name, n)| format!("{name} {n}")).collect::<Vec<_>>().join(", ")
        };
        println!("  structures: kind, precision (hits/placed; non-air only), recall (found/wanted structure blocks), BDS blocks wanted");
        for (kind, c) in &self.kinds {
            let starts = self.starts.get(kind).map_or(String::new(), |s| format!(" starts {s:?}"));
            println!(
                "    {kind:>16}: precision {:7.3}% ({}/{}; {:.3}% {}/{}), recall {:7.3}% ({}/{}){starts}",
                percent(c.hits, c.placed),
                c.hits,
                c.placed,
                percent(c.solid_hits, c.solid_placed),
                c.solid_hits,
                c.solid_placed,
                percent(c.found, c.wanted),
                c.found,
                c.wanted
            );
            if !c.blocks.is_empty() {
                println!("      {}", top(&c.blocks));
            }
        }
        let unattributed: u64 = self.unattributed.values().sum();
        println!("    unattributed structure-looking cells: {unattributed}: {}", top(&self.unattributed));
    }
}
