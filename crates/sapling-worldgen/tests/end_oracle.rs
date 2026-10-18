//! The End against a one-worker, seed-0 BDS canon (tools/worldgen-oracle/README.md, "The End"): run with
//! `--ignored --nocapture`. Needs `$SAPLING_ORACLE_WORLDS/<world>`, its `-run2` twin and `<world>-buffers`; with
//! `order.txt` (and `structure-order.txt`) beside them the decoration (and the structure stage) replay that run.
//! Lines as tests/nether_oracle.rs: `terrain` counts cells where our loaded chunk differs from the buffer `loadChunk`
//! committed; past that a cell is `structures` (an end city block on either side) or `features`.

mod nether_support;

use std::collections::BTreeMap;
use std::path::PathBuf;

use nether_support::{Buffers, index};
use sapling_leveldb::{ChunkKey, LevelDbWorld};
use sapling_worldgen::EndGenerator;

struct Region {
    name: &'static str,
    chunks: [i32; 4],
}

const REGIONS: &[Region] = &[
    Region { name: "main island", chunks: [-8, -8, 7, 7] },
    Region { name: "end city with a ship", chunks: [82, -2, 87, 9] },
    Region { name: "outer islands", chunks: [60, -4, 67, 3] },
    Region { name: "void", chunks: [20, -2, 23, 1] },
];

const STAGES: [&str; 4] = ["terrain", "biomes", "structures", "features"];
/// Above the buffer's 128 rows the loaded chunk is air; features and structures reach up to y 255.
const HEIGHT: i32 = 256;

#[derive(Default)]
struct Tally {
    cells: u64,
    unstable: u64,
    missing: u64,
    wrong: BTreeMap<&'static str, u64>,
    pairs: BTreeMap<(&'static str, String, String), u64>,
    chunks: BTreeMap<(i32, i32), u64>,
}

impl Tally {
    fn wrong(&mut self, stage: &'static str, ours: &str, bds: &str) {
        *self.wrong.entry(stage).or_default() += 1;
        *self.pairs.entry((stage, ours.to_string(), bds.to_string())).or_default() += 1;
    }

    fn print(&self) {
        let wrong: u64 = self.wrong.iter().filter(|(stage, _)| **stage != "biomes").map(|(_, n)| n).sum();
        let identical = 100.0 * (self.cells - wrong) as f64 / self.cells as f64;
        println!("  exact blocks: {} stable cells, {identical:.4}% identical, {wrong} wrong, 0 left to later stages; {} unstable between BDS runs", self.cells, self.unstable);
        println!("  missing: {} BDS cells set where ours kept the loaded block", self.missing);
        let stages: Vec<String> = STAGES.iter().map(|s| format!("{s} {}", self.wrong.get(s).copied().unwrap_or(0))).collect();
        println!("  stages: {}", stages.join(", "));
        let mut pairs: Vec<_> = self.pairs.iter().collect();
        pairs.sort_by(|a, b| b.1.cmp(a.1));
        for ((stage, ours, bds), count) in pairs.iter().take(16) {
            println!("    {count:>8} {stage}: {ours} / {bds}");
        }
        let mut chunks: Vec<_> = self.chunks.iter().collect();
        chunks.sort_by(|a, b| b.1.cmp(a.1));
        let top: Vec<String> = chunks.iter().take(12).map(|((x, z), n)| format!("{x},{z} {n}")).collect();
        println!("    by chunk: {}", top.join(", "));
    }
}

fn worlds() -> PathBuf {
    std::env::var_os("SAPLING_ORACLE_WORLDS").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../worlds/oracle"))
}

/// Blocks only end cities and their ships place.
fn is_structure_block(name: &str) -> bool {
    ["purpur", "end_brick", "stained_glass"].iter().any(|part| name.contains(part))
        || matches!(name, "end_rod" | "ladder" | "wall_banner" | "chest" | "ender_chest" | "frame" | "brewing_stand" | "dragon_head" | "skull")
}

fn compare(seed: i64, world: &str) {
    let root = worlds();
    let open = |name: String| LevelDbWorld::open_in_memory(&root.join(name)).ok();
    let (Some(primary), Some(twin)) = (open(world.to_string()), open(format!("{world}-run2"))) else {
        eprintln!("oracle world {world} missing; skipped");
        return;
    };
    let buffers = Buffers::open_named(&root.join(format!("{world}-buffers")), "end").expect("the canon's buffers");
    let generator = EndGenerator::new(seed);
    let only = std::env::var("SAPLING_ORACLE_REGION").ok();
    let kept = |name: &str| only.as_deref().is_none_or(|only| only.strip_prefix('=').map_or(name.contains(only), |exact| name == exact));
    let registry = primary.registry();
    let name = |id: u32| registry.get(id).map_or("?", |s| s.name.trim_start_matches("minecraft:"));
    for region in REGIONS.iter().filter(|r| kept(r.name)) {
        println!("seed {seed} {world} {}:", region.name);
        let mut tally = Tally::default();
        let [x0, z0, x1, z1] = region.chunks;
        for (cx, cz) in (x0..=x1).flat_map(|x| (z0..=z1).map(move |z| (x, z))) {
            let key = ChunkKey::new(cx, cz, 2);
            let a = primary.load_column(key).unwrap().expect("chunk saved").chunk;
            let b = twin.load_column(key).unwrap().expect("twin chunk saved").chunk;
            let buffer = buffers.chunk(cx, cz).expect("a dumped buffer for every region chunk");
            let loaded = generator.terrain_blocks(cx, cz);
            let finished = generator.chunk_blocks(cx, cz);
            for z in 0..16 {
                for x in 0..16 {
                    let (wx, wz) = (cx * 16 + x as i32, cz * 16 + z as i32);
                    if a.biome(wx, 0, wz) != Some(u32::from(sapling_worldgen::end::THE_END)) {
                        tally.wrong("biomes", "the_end", &format!("{:?}", a.biome(wx, 0, wz)));
                    }
                    for y in 0..HEIGHT {
                        let saved = (a.block(wx, y, wz), a.liquid(wx, y, wz));
                        if saved != (b.block(wx, y, wz), b.liquid(wx, y, wz)) {
                            tally.unstable += 1;
                            continue;
                        }
                        tally.cells += 1;
                        let committed = if y < 128 { buffer[index(x, y, z)] } else { "air" };
                        let (base, ours, bds) = (loaded.block(x, y, z).name(), finished.block(x, y, z), name(saved.0));
                        let bds_states = registry.get(saved.0).map_or(String::new(), |s| s.properties.to_string());
                        if base != committed {
                            tally.wrong("terrain", base, committed);
                        } else if ours.name() != bds || ours.states() != bds_states {
                            tally.missing += u64::from(ours.name() == base && ours.name() != bds);
                            let (ours_full, bds_full) = (format!("{}[{}]", ours.name(), ours.states()), format!("{bds}[{bds_states}]"));
                            let stage = if is_structure_block(ours.name()) || is_structure_block(bds) { "structures" } else { "features" };
                            tally.wrong(stage, &ours_full, &bds_full);
                            *tally.chunks.entry((cx, cz)).or_default() += 1;
                        }
                    }
                }
            }
        }
        tally.print();
    }
}

#[test]
#[ignore]
fn end_matches_bds() {
    compare(20261003, "20261003-end");
}
