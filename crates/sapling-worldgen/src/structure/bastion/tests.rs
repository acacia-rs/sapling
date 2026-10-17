use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use sapling_leveldb::{ChunkKey, LevelDbWorld};

use super::Bastion;
use crate::nether::NetherGenerator;
use crate::random::{LargeFeatureSeed, Mt19937};
use crate::structure::BoundingBox;
use crate::block::Block;
use crate::structure::canvas::Canvas;

const SEED: i64 = 20261003;

/// The shared Nether selector's region generator after its draws (1480b9cf0, salt 30084232), and the start chunk.
fn region(rx: i32, rz: i32) -> (Mt19937, (i32, i32), bool) {
    let seed = (rz as u32).wrapping_mul(0xF1565BD5).wrapping_add((rx as u32).wrapping_mul(0x9939F508)).wrapping_add(SEED as u32).wrapping_add(0x1CB0C88);
    let mut random = Mt19937::new(seed);
    let (x, z) = (random.next_int(26), random.next_int(26));
    let bastion = random.next_int(6) >= 2;
    (random, (rx * 30 + x, rz * 30 + z), bastion)
}

#[test]
fn bastion_starts_at_y_32_and_stays_in_reach() {
    let (mut random, start, bastion) = region(-1, 0);
    assert!(bastion);
    let b = Bastion::generate(&mut random, start);
    assert_eq!(b.pieces[0].bb.y0, 32);
    assert!(b.pieces.len() > 1);
    let reach = BoundingBox::new((start.0 - super::REACH) * 16, i32::MIN, (start.1 - super::REACH) * 16, (start.0 + super::REACH) * 16 + 15, i32::MAX, (start.1 + super::REACH) * 16 + 15);
    assert!(b.pieces.iter().all(|p| reach.contains(p.bb.x0, 0, p.bb.z0) && reach.contains(p.bb.x1, 0, p.bb.z1)));
}

fn worlds() -> PathBuf {
    std::env::var_os("SAPLING_ORACLE_WORLDS").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../worlds/oracle"))
}

/// Post-process order of the canon's run: `x z` lines of structure-order.txt.
fn order(chunks: &[(i32, i32)]) -> Vec<(i32, i32)> {
    let text = std::fs::read_to_string(worlds().join("structure-order.txt")).unwrap_or_default();
    let mut logged: Vec<(i32, i32)> = text.lines().filter(|l| !l.starts_with("load")).filter_map(|l| l.split_once(' ')).map(|(x, z)| (x.parse().unwrap(), z.parse().unwrap())).filter(|c| chunks.contains(c)).collect();
    logged.dedup();
    let rest: Vec<(i32, i32)> = chunks.iter().filter(|c| !logged.contains(c)).copied().collect();
    logged.extend(rest);
    logged
}

/// The canon's bastion (region -1,0, start -21,15) over carved chunks against the saved world, cells
/// that either side has as bastion: `SAPLING_ORACLE_WORLDS=<scratchpad>/nether/worlds`, `--ignored --nocapture`.
#[test]
#[ignore]
fn bastion_matches_canon() {
    let Ok(world) = LevelDbWorld::open_in_memory(&worlds().join("20261003-nether")) else { return eprintln!("no canon; skipped") };
    let registry = world.registry();
    let (mut random, start, _) = region(-1, 0);
    let bastion = Bastion::generate(&mut random, start);
    for p in &bastion.pieces {
        println!("piece {} {:?} {:?} {:?} depth {}", p.element.name, p.origin, p.rotation, p.bb, p.depth);
    }
    let generator = NetherGenerator::new(SEED);
    let seeds = LargeFeatureSeed::new(SEED as u32);
    let chunks: Vec<(i32, i32)> = (-24..=-18).flat_map(|x| (12..=18).map(move |z| (x, z))).collect();
    let mut overlay: HashMap<(i32, i32, i32), Block> = HashMap::new();
    let mut shapes = HashMap::new();
    for chunk in order(&chunks) {
        let mut shape = generator.carved_blocks(chunk.0, chunk.1);
        for (&(x, y, z), &b) in overlay.iter().filter(|((x, _, z), _)| (x >> 4, z >> 4) == chunk) {
            shape.set_block((x & 15) as usize, y, (z & 15) as usize, b);
        }
        let neighbour = |x: i32, y: i32, z: i32| overlay.get(&(x, y, z)).copied().unwrap_or_else(|| generator.carved_blocks(x >> 4, z >> 4).block((x & 15) as usize, y, (z & 15) as usize));
        let mut canvas = Canvas::new(&mut shape, chunk, &neighbour);
        super::place(&mut canvas, &[&bastion], seeds.chunk(chunk.0, chunk.1));
        let past = std::mem::take(&mut canvas.unclipped);
        drop(canvas);
        overlay.extend(past.into_iter().map(|([x, y, z], b)| ((x, y, z), b)));
        shapes.insert(chunk, shape);
    }
    let (mut same, mut wrong, mut missing) = (0u64, 0u64, 0u64);
    let mut pairs: BTreeMap<(String, String), u64> = BTreeMap::new();
    let mut by_chunk: BTreeMap<(i32, i32), u64> = BTreeMap::new();
    let in_piece = |x: i32, y: i32, z: i32| bastion.pieces.iter().any(|p| p.bb.contains(x, y, z));
    for &(cx, cz) in &chunks {
        let saved = world.load_column(ChunkKey::new(cx, cz, 1)).unwrap().expect("chunk saved").chunk;
        let carved = generator.carved_blocks(cx, cz);
        let ours = &shapes[&(cx, cz)];
        for (x, z) in (0..16).flat_map(|x| (0..16).map(move |z| (x, z))) {
            for y in 0..128 {
                let (wx, wz) = (cx * 16 + x as i32, cz * 16 + z as i32);
                let state = registry.get(saved.block(wx, y, wz)).expect("a saved state");
                let (bds, bds_states) = (state.name.trim_start_matches("minecraft:"), state.properties.to_string());
                let (o, c) = (ours.block(x, y, z), carved.block(x, y, z));
                let matches = o.name() == bds && o.states() == bds_states;
                if o != c || in_piece(wx, y, wz) {
                    if matches {
                        same += 1;
                        continue;
                    }
                    missing += u64::from(o == c);
                    wrong += 1;
                    *pairs.entry((format!("{}[{}]", o.name(), o.states()), format!("{bds}[{bds_states}]"))).or_default() += 1;
                    *by_chunk.entry((cx, cz)).or_default() += 1;
                }
            }
        }
    }
    for p in bastion.pieces.iter().filter(|p| p.element.name.starts_with("bastion_mobs")) {
        let (x, y, z) = p.origin;
        let Some(saved) = world.load_column(ChunkKey::new(x >> 4, z >> 4, 1)).unwrap() else { continue };
        let names: Vec<_> = (y - 1..y + 3).map(|y| registry.get(saved.chunk.block(x, y, z)).map_or("?".to_string(), |s| s.name.to_string())).collect();
        println!("mob {} at {:?}: below..top {:?}", p.element.name, p.origin, names);
    }
    println!("bastion cells: {same} same, {wrong} wrong ({missing} BDS set where ours kept the carved block)");
    let mut top: Vec<_> = pairs.into_iter().collect();
    top.sort_by(|a, b| b.1.cmp(&a.1));
    for ((o, b), n) in top.iter().take(40) {
        println!("  {n:>7} ours {o} / bds {b}");
    }
    println!("  by chunk: {:?}", by_chunk);
}
