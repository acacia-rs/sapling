//! The Nether against a one-worker, seed-0 BDS canon (tools/worldgen-oracle/README.md, "The Nether"): run with
//! `--ignored --nocapture`. Needs `$SAPLING_ORACLE_WORLDS/<world>`, its `-run2` twin and `<world>-buffers`; with
//! `order.txt` (and `structure-order.txt`) beside them the decoration (and the structure stage) replay that run.
//! Each region prints the oracle test's `exact blocks` and `missing` lines over our finished chunks: `wrong` is every
//! differing cell, `missing` those where ours kept the loaded block and BDS set another (a feature or structure we
//! lack). The `stages` line splits `wrong`: a cell the loaded buffer already gets wrong is the stage of ours that
//! decided it (`Stages::stage_of`); after that, liquids (lava or air against lava), structures (a structure block on
//! either side), else features. `SAPLING_ORACLE_REGION` as in tests/oracle.rs.

mod nether_support;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use nether_support::{Buffers, Stages, index, is_structure_block};
use sapling_leveldb::{ChunkKey, LevelDbWorld};
use sapling_worldgen::{NetherGenerator, StructureStep};

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

/// The second canon's (`tools/nether-replay.sh <name> nether-gaps`): two ruined portals and soul sand valley fossils.
const GAP_REGIONS: &[Region] = &[
    Region { name: "ruined portal west", chunks: [-18, -2, -15, 1] },
    Region { name: "ruined portal north-west", chunks: [-21, -17, -19, -15] },
    Region { name: "soul sand valley fossils", chunks: [-8, 8, -4, 11] },
];

const STAGES: [&str; 8] = ["shape", "bedrock", "surface", "carvers", "biomes", "structures", "features", "liquids"];

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

/// The canon's logged run, if it has one: `structure-order.txt` (`load x z` and `x z` lines, scripts/dbg_loads.py
/// --structure-order) and `order.txt` (`x z [draws]` per line, --order).
fn replay(generator: &mut NetherGenerator, root: &Path) {
    let numbers = |line: &str| -> (i32, i32) {
        let v: Vec<i32> = line.split(' ').map(|v| v.parse().expect("an integer")).collect();
        (v[0], v[1])
    };
    if let Ok(text) = std::fs::read_to_string(root.join("structure-order.txt")) {
        let step = |line: &str| line.strip_prefix("load ").map_or_else(|| StructureStep::Post(numbers(line)), |chunk| StructureStep::Load(numbers(chunk)));
        let steps: Vec<StructureStep> = text.lines().map(step).collect();
        generator.set_structure_order(&steps);
    }
    let Ok(text) = std::fs::read_to_string(root.join("order.txt")) else { return };
    let rows: Vec<Vec<i64>> = text.lines().map(|line| line.split(' ').map(|v| v.parse().expect("an integer")).collect()).collect();
    let order: Vec<(i32, i32)> = rows.iter().map(|row| (row[0] as i32, row[1] as i32)).collect();
    let spent: Vec<Option<u32>> = rows.iter().map(|row| row.get(2).map(|&draws| draws as u32)).collect();
    generator.set_decoration_order(&order, Some((0, &spent)));
}

/// What cell of BDS's a differing cell of ours belongs to, past the loaded buffer.
fn later_stage(ours: &str, bds: &str) -> &'static str {
    let liquid = |name: &str| name == "lava";
    let open = |name: &str| name == "air" || liquid(name);
    if (liquid(ours) || liquid(bds)) && open(ours) && open(bds) {
        "liquids"
    } else if is_structure_block(ours) || is_structure_block(bds) {
        "structures"
    } else {
        "features"
    }
}

fn compare(seed: i64, world: &str, regions: &[Region]) {
    let root = worlds();
    let open = |name: String| LevelDbWorld::open_in_memory(&root.join(name)).ok();
    let (Some(primary), Some(twin)) = (open(world.to_string()), open(format!("{world}-run2"))) else {
        eprintln!("oracle world {world} missing; skipped");
        return;
    };
    let buffers = Buffers::open(&root.join(format!("{world}-buffers"))).expect("the canon's buffers");
    let mut generator = NetherGenerator::new(seed);
    replay(&mut generator, &root);
    let only = std::env::var("SAPLING_ORACLE_REGION").ok();
    let kept = |name: &str| only.as_deref().is_none_or(|only| only.strip_prefix('=').map_or(name.contains(only), |exact| name == exact));
    let registry = primary.registry();
    let name = |id: u32| registry.get(id).map_or("?", |s| s.name.trim_start_matches("minecraft:"));
    for region in regions.iter().filter(|r| kept(r.name)) {
        println!("seed {seed} {world} {}:", region.name);
        let mut tally = Tally::default();
        let [x0, z0, x1, z1] = region.chunks;
        for (cx, cz) in (x0..=x1).flat_map(|x| (z0..=z1).map(move |z| (x, z))) {
            let key = ChunkKey::new(cx, cz, 1);
            let a = primary.load_column(key).unwrap().expect("chunk saved").chunk;
            let b = twin.load_column(key).unwrap().expect("twin chunk saved").chunk;
            let buffer = buffers.chunk(cx, cz).expect("a dumped buffer for every region chunk");
            let stages = Stages::new(&generator, cx, cz);
            let finished = generator.chunk_blocks(cx, cz);
            let biomes = generator.chunk_biomes(cx, cz);
            for z in 0..16 {
                for x in 0..16 {
                    let (wx, wz) = (cx * 16 + x as i32, cz * 16 + z as i32);
                    if a.biome(wx, 0, wz) != Some(biomes.biome(x, 0, z)) {
                        tally.wrong("biomes", &biomes.biome(x, 0, z).to_string(), &format!("{:?}", a.biome(wx, 0, wz)));
                    }
                    for y in 0..128 {
                        let saved = (a.block(wx, y, wz), a.liquid(wx, y, wz));
                        if saved != (b.block(wx, y, wz), b.liquid(wx, y, wz)) {
                            tally.unstable += 1;
                            continue;
                        }
                        tally.cells += 1;
                        let (loaded, carved, ours, bds) = (buffer[index(x, y, z)], stages.block(x, y, z).name(), finished.block(x, y, z), name(saved.0));
                        let bds_states = registry.get(saved.0).map_or(String::new(), |s| s.properties.to_string());
                        if carved != loaded {
                            tally.wrong(stages.stage_of(x, y, z, loaded), carved, loaded);
                        } else if ours.name() != bds || ours.states() != bds_states {
                            tally.missing += u64::from(ours.name() == carved && ours.name() != bds);
                            let (ours_full, bds_full) = (format!("{}[{}]", ours.name(), ours.states()), format!("{bds}[{bds_states}]"));
                            tally.wrong(later_stage(ours.name(), bds), &ours_full, &bds_full);                            *tally.chunks.entry((cx, cz)).or_default() += 1;
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
    compare(20261003, "20261003-nether", REGIONS);
}

#[test]
#[ignore]
fn nether_gaps_match_bds() {
    compare(20261003, "20261003-nether-gaps", GAP_REGIONS);
}
