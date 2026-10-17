//! The Nether against a one-worker, seed-0 BDS canon (tools/worldgen-oracle/README.md, "The Nether"): run with
//! `--ignored --nocapture`. Needs `$SAPLING_ORACLE_WORLDS/<world>`, its `-run2` twin and `<world>-buffers`.
//! Each region prints the oracle test's `exact blocks` and `missing` lines: `wrong` counts the cells our ported
//! stages decide (the loaded buffer and the biomes), split by the stage of ours that wrote them; `left to later
//! stages` the cells BDS changed after `loadChunk` (features, structures), which `missing` repeats.
//! `SAPLING_ORACLE_REGION` as in tests/oracle.rs.

mod nether_support;

use std::collections::BTreeMap;
use std::path::PathBuf;

use nether_support::{Buffers, Stages, index, is_structure_block};
use sapling_leveldb::{ChunkKey, LevelDbWorld};
use sapling_worldgen::NetherGenerator;

struct Region {
    name: &'static str,
    chunks: [i32; 4],
}

const REGIONS: &[Region] = &[
    Region { name: "nether wastes and basalt deltas", chunks: [-4, -4, 3, 3] },
    Region { name: "warped forest", chunks: [-24, 0, -21, 3] },
    Region { name: "crimson forest", chunks: [-24, -24, -21, -21] },
    Region { name: "soul sand valley", chunks: [-8, -28, -5, -25] },
    Region { name: "fortress", chunks: [12, -1, 19, 6] },
    Region { name: "bastion", chunks: [-24, 12, -18, 18] },
];

#[derive(Default)]
struct Tally {
    cells: u64,
    unstable: u64,
    wrong: BTreeMap<&'static str, u64>,
    pairs: BTreeMap<(&'static str, String, String), u64>,
    later: BTreeMap<String, u64>,
    unjudged: u64,
}

impl Tally {
    fn wrong(&mut self, stage: &'static str, ours: &str, bds: &str) {
        *self.wrong.entry(stage).or_default() += 1;
        *self.pairs.entry((stage, ours.to_string(), bds.to_string())).or_default() += 1;
    }

    fn print(&self) {
        let wrong: u64 = self.wrong.values().sum();
        let later: u64 = self.later.values().sum();
        let identical = 100.0 * (self.cells - wrong - later) as f64 / self.cells as f64;
        println!("  exact blocks: {} stable cells, {identical:.4}% identical, {wrong} wrong, {later} left to later stages; {} unstable between BDS runs", self.cells, self.unstable);
        println!("  missing: {later} BDS cells set after loadChunk; {} cells without a dumped buffer", self.unjudged);
        let stages: Vec<String> = ["shape", "bedrock", "surface", "carvers", "biomes"].iter().map(|s| format!("{s} {}", self.wrong.get(s).copied().unwrap_or(0))).collect();
        let structures: u64 = self.later.iter().filter(|(name, _)| is_structure_block(name)).map(|(_, n)| n).sum();
        println!("  stages: {}; later: structures {structures}, features {}", stages.join(", "), later - structures);
        let mut pairs: Vec<_> = self.pairs.iter().collect();
        pairs.sort_by(|a, b| b.1.cmp(a.1));
        for ((stage, ours, bds), count) in pairs.iter().take(12) {
            println!("    {count:>8} {stage}: {ours} / {bds}");
        }
        let mut later: Vec<_> = self.later.iter().collect();
        later.sort_by(|a, b| b.1.cmp(a.1));
        let top: Vec<String> = later.iter().take(12).map(|(name, n)| format!("{name} {n}")).collect();
        println!("    later, by BDS block: {}", top.join(", "));
    }
}

fn worlds() -> PathBuf {
    std::env::var_os("SAPLING_ORACLE_WORLDS").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../worlds/oracle"))
}

fn compare(seed: i64, world: &str) {
    let root = worlds();
    let open = |name: String| LevelDbWorld::open_in_memory(&root.join(name)).ok();
    let (Some(primary), Some(twin)) = (open(world.to_string()), open(format!("{world}-run2"))) else {
        eprintln!("oracle world {world} missing; skipped");
        return;
    };
    let buffers = Buffers::open(&root.join(format!("{world}-buffers")));
    let generator = NetherGenerator::new(seed);
    let only = std::env::var("SAPLING_ORACLE_REGION").ok();
    let kept = |name: &str| only.as_deref().is_none_or(|only| only.strip_prefix('=').map_or(name.contains(only), |exact| name == exact));
    let registry = primary.registry();
    let name = |id: u32| registry.get(id).map_or("?", |s| s.name.trim_start_matches("minecraft:"));
    for region in REGIONS.iter().filter(|r| kept(r.name)) {
        println!("seed {seed} {world} {}:", region.name);
        let mut tally = Tally::default();
        let [x0, z0, x1, z1] = region.chunks;
        for (cx, cz) in (x0..=x1).flat_map(|x| (z0..=z1).map(move |z| (x, z))) {
            let key = ChunkKey::new(cx, cz, 1);
            let a = primary.load_column(key).unwrap().expect("chunk saved").chunk;
            let b = twin.load_column(key).unwrap().expect("twin chunk saved").chunk;
            let buffer = buffers.as_ref().and_then(|buffers| buffers.chunk(cx, cz));
            let stages = Stages::new(&generator, cx, cz);
            let biomes = generator.chunk_biomes(cx, cz);
            for z in 0..16 {
                for x in 0..16 {
                    let (wx, wz) = (cx * 16 + x as i32, cz * 16 + z as i32);
                    if a.biome(wx, 0, wz) != Some(biomes.biome(x, 0, z)) {
                        tally.wrong("biomes", &biomes.biome(x, 0, z).to_string(), &format!("{:?}", a.biome(wx, 0, wz)));
                    }
                    for y in 0..128 {
                        let (saved, other) = ((a.block(wx, y, wz), a.liquid(wx, y, wz)), (b.block(wx, y, wz), b.liquid(wx, y, wz)));
                        if saved != other {
                            tally.unstable += 1;
                            continue;
                        }
                        tally.cells += 1;
                        let ours = stages.block(x, y, z).name();
                        let Some(buffer) = &buffer else {
                            tally.unjudged += u64::from(ours != name(saved.0));
                            continue;
                        };
                        let loaded = buffer[index(x, y, z)];
                        if ours != loaded {
                            // A cell only the density decided: BDS's air in our netherrack is a carver's, another
                            // block a surface builder's.
                            let stage = match stages.writer(x, y, z) {
                                "shape" if ours == "netherrack" && matches!(loaded, "air" | "lava") => "carvers",
                                "shape" if ours == "netherrack" => "surface",
                                stage => stage,
                            };
                            tally.wrong(stage, ours, loaded);
                        } else if ours != name(saved.0) {
                            *tally.later.entry(name(saved.0).to_string()).or_default() += 1;
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
fn nether_matches_bds() {
    compare(20261003, "20261003-nether");
}
