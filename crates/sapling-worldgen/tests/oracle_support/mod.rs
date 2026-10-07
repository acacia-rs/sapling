//! Reading oracle worlds and summarising where sapling and BDS disagree.

mod carver;

pub use carver::CarverTally;

use std::collections::BTreeMap;
use std::path::PathBuf;

use acacia_world::Chunk;
use sapling_leveldb::LevelDbWorld;
use sapling_worldgen::CellKind;

pub struct Region {
    pub name: &'static str,
    x0: i32,
    z0: i32,
    x1: i32,
    z1: i32,
}

impl Region {
    pub fn new(name: &'static str, x0: i32, z0: i32, x1: i32, z1: i32) -> Self {
        Self { name, x0, z0, x1, z1 }
    }

    pub fn chunks(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        (self.x0..=self.x1).flat_map(move |x| (self.z0..=self.z1).map(move |z| (x, z)))
    }
}

/// The world and its `-run2` twin, or None when the oracle worlds are absent.
pub fn open(world: &str) -> Option<(LevelDbWorld, LevelDbWorld)> {
    let root = std::env::var_os("SAPLING_ORACLE_WORLDS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../worlds/oracle"));
    let (a, b) = (root.join(world), root.join(format!("{world}-run2")));
    if !a.exists() || !b.exists() {
        return None;
    }
    Some((LevelDbWorld::open_in_memory(&a).unwrap(), LevelDbWorld::open_in_memory(&b).unwrap()))
}

/// Blocks the terrain stages (shape, surface, ore veins, deepslate) place; any other solid block
/// is decoration (features, structures, carvers' side effects).
const TERRAIN_BLOCKS: &[&str] = &[
    "stone", "deepslate", "granite", "diorite", "andesite", "tuff", "dirt", "grass_block", "coarse_dirt", "podzol", "mycelium",
    "sand", "red_sand", "sandstone", "red_sandstone", "gravel", "clay", "mud", "snow", "powder_snow", "ice", "packed_ice",
    "blue_ice", "bedrock", "terracotta", "white_terracotta", "orange_terracotta", "yellow_terracotta",
    "brown_terracotta", "red_terracotta", "light_gray_terracotta", "copper_ore", "deepslate_copper_ore", "raw_copper_block",
    "iron_ore", "deepslate_iron_ore", "raw_iron_block",
];

pub struct BdsCell {
    pub kind: CellKind,
    pub block: String,
}

/// Air, water, lava or solid; a waterlogged block counts as water.
pub fn bds_cell(world: &LevelDbWorld, chunk: &Chunk, x: i32, y: i32, z: i32) -> BdsCell {
    let registry = world.registry();
    let name = |id| registry.get(id).map_or("?", |state| state.name.trim_start_matches("minecraft:"));
    let block = name(chunk.block(x, y, z));
    let kind = match block {
        "air" | "cave_air" | "void_air" => CellKind::Air,
        "water" | "flowing_water" => CellKind::Water,
        "lava" | "flowing_lava" => CellKind::Lava,
        _ if matches!(name(chunk.liquid(x, y, z)), "water" | "flowing_water") => CellKind::Water,
        _ => CellKind::Solid,
    };
    BdsCell { kind, block: block.to_string() }
}

/// Why a cell differs, judged from the pair alone.
fn explanation(ours: CellKind, bds: &BdsCell) -> &'static str {
    match (ours, bds.kind) {
        (CellKind::Solid, _) => "carved by BDS (carvers, lakes)",
        (_, CellKind::Solid) if TERRAIN_BLOCKS.contains(&bds.block.as_str()) => "shape: BDS terrain block where ours is empty",
        (_, CellKind::Solid) => "decoration (features)",
        _ => "fluid",
    }
}

#[derive(Default)]
struct Group {
    count: u64,
    first: Option<(i32, i32, i32)>,
    by_band: BTreeMap<i32, u64>,
    blocks: BTreeMap<String, u64>,
}

#[derive(Default)]
pub struct Tally {
    matched: u64,
    unstable: u64,
    groups: BTreeMap<(&'static str, String), Group>,
}

impl Tally {
    pub fn add(&mut self, ours: CellKind, bds: &BdsCell, stable: bool, at: (i32, i32, i32)) {
        if !stable {
            self.unstable += 1;
            return;
        }
        if ours == bds.kind {
            self.matched += 1;
            return;
        }
        let group = self.groups.entry((explanation(ours, bds), format!("ours {ours:?} bds {:?}", bds.kind))).or_default();
        group.count += 1;
        group.first.get_or_insert(at);
        *group.by_band.entry(at.1.div_euclid(32) * 32).or_default() += 1;
        *group.blocks.entry(bds.block.clone()).or_default() += 1;
    }

    pub fn print(&self) {
        let mismatched: u64 = self.groups.values().map(|g| g.count).sum();
        let compared = self.matched + mismatched;
        println!(
            "  {compared} stable cells, {:.4}% agree, {mismatched} differ; {} cells unstable between BDS runs",
            100.0 * self.matched as f64 / compared.max(1) as f64,
            self.unstable
        );
        for ((why, pair), group) in &self.groups {
            let bands: Vec<String> = group.by_band.iter().map(|(y, n)| format!("{y}:{n}")).collect();
            let mut blocks: Vec<_> = group.blocks.iter().collect();
            blocks.sort_by(|a, b| b.1.cmp(a.1));
            let blocks: Vec<String> = blocks.iter().take(5).map(|(b, n)| format!("{b}:{n}")).collect();
            println!("  {why}: {pair} {:>8}  first {:?}", group.count, group.first.unwrap());
            println!("      by y/32 {}  | {}", bands.join(" "), blocks.join(" "));
        }
    }
}

impl BdsCell {
    /// Water, or a solid block the terrain stages place.
    pub fn is_terrain(&self) -> bool {
        self.kind == CellKind::Water || (self.kind == CellKind::Solid && TERRAIN_BLOCKS.contains(&self.block.as_str()))
    }
}

/// Per-block biome agreement, with the most common (ours, BDS) disagreements.
#[derive(Default)]
pub struct BiomeTally {
    matched: u64,
    pairs: BTreeMap<(u32, Option<u32>), (u64, (i32, i32, i32))>,
}

impl BiomeTally {
    pub fn add(&mut self, ours: u32, bds: Option<u32>, at: (i32, i32, i32)) {
        if Some(ours) == bds {
            self.matched += 1;
        } else {
            self.pairs.entry((ours, bds)).or_insert((0, at)).0 += 1;
        }
    }

    pub fn print(&self) {
        let mismatched: u64 = self.pairs.values().map(|p| p.0).sum();
        let total = self.matched + mismatched;
        println!("  biomes: {total} blocks, {:.4}% agree, {mismatched} differ", 100.0 * self.matched as f64 / total.max(1) as f64);
        let mut pairs: Vec<_> = self.pairs.iter().collect();
        pairs.sort_by(|a, b| b.1.0.cmp(&a.1.0));
        for ((ours, bds), (count, at)) in pairs.iter().take(12) {
            println!("    ours {ours:>3} bds {bds:?} {count:>9}  first {at:?}");
        }
    }
}
