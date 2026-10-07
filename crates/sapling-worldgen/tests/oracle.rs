//! Terrain shape against BDS-saved oracle worlds (tools/worldgen-oracle/README.md). Needs the
//! git-ignored `worlds/oracle/`; run with `--ignored --nocapture`.
//! Cells where the world and its `-run2` twin disagree are counted apart, not as mismatches.

mod oracle_support;

use oracle_support::{BiomeTally, CarverTally, Ours, Region, Tally, bds_cell, open};
use sapling_leveldb::ChunkKey;
use std::collections::BTreeMap;

use sapling_worldgen::{CellKind, MIN_Y, OverworldGenerator};

/// The bedrock floor (y -64..-60) is not generated yet; compared from y -59.
const FIRST_Y: i32 = -59;
const LAST_Y: i32 = 319;

fn compare(seed: i64, world: &str, regions: &[Region]) {
    let Some((primary, twin)) = open(world) else {
        eprintln!("oracle world {world} missing; skipped");
        return;
    };
    let generator = OverworldGenerator::new(seed);
    for region in regions {
        let mut tally = Tally::default();
        let mut biome_tally = BiomeTally::default();
        let mut carver_tally = CarverTally::default();
        // Our highest non-air cell minus BDS's highest terrain cell, per column.
        let mut top_offsets: BTreeMap<i32, (u64, (i32, i32))> = BTreeMap::new();
        for (cx, cz) in region.chunks() {
            let key = ChunkKey::new(cx, cz, 0);
            let a = primary.load_column(key).unwrap().expect("chunk saved").chunk;
            let b = twin.load_column(key).unwrap().expect("twin chunk saved").chunk;
            let uncarved = generator.chunk_shape(cx, cz);
            let mut ours = uncarved.clone();
            generator.carve(&mut ours, cx, cz);
            generator.decorate(&mut ours, cx, cz);
            let biomes = generator.chunk_biomes(cx, cz);
            let biome_top = highest_section_top(&primary, &a, cx, cz);
            for z in 0..16 {
                for x in 0..16 {
                    let (wx, wz) = (cx * 16 + x as i32, cz * 16 + z as i32);
                    let (mut our_top, mut bds_top) = (None, None);
                    for y in MIN_Y..=biome_top {
                        biome_tally.add(biomes.biome(x, y, z), a.biome(wx, y, wz), (wx, y, wz));
                    }
                    for y in (FIRST_Y..=LAST_Y).rev() {
                        let bds = bds_cell(&primary, &a, wx, y, wz);
                        let stable = bds.kind == bds_cell(&twin, &b, wx, y, wz).kind;
                        let kind = ours.kind(x, y, z);
                        tally.add(kind, &bds, stable, (wx, y, wz));
                        if stable {
                            let next_to_cave = [y - 1, y + 1].iter().any(|&ny| ny <= LAST_Y && uncarved.kind(x, ny, z) != CellKind::Solid);
                            let cell = Ours { before: uncarved.kind(x, y, z), after: kind, next_to_cave };
                            carver_tally.add(cell, &bds, (wx, y, wz), &primary, &a);
                        }
                        if our_top.is_none() && kind != CellKind::Air {
                            our_top = Some(y);
                        }
                        if bds_top.is_none() && bds.is_terrain() {
                            bds_top = Some(y);
                        }
                    }
                    top_offsets.entry(our_top.unwrap_or(0) - bds_top.unwrap_or(0)).or_insert((0, (wx, wz))).0 += 1;
                }
            }
        }
        println!("seed {seed} {}:", region.name);
        tally.print();
        carver_tally.print();
        biome_tally.print();
        let offsets: Vec<String> = top_offsets.iter().map(|(d, (n, at))| format!("{d:+}:{n} {at:?}")).collect();
        println!("  top of terrain, ours minus BDS, columns: {}", offsets.join(" "));
    }
}

/// BDS saves the biomes of sub-chunks above the highest non-empty one as copies of it, so
/// biomes are compared up to the top of that sub-chunk only.
fn highest_section_top(world: &sapling_leveldb::LevelDbWorld, chunk: &acacia_world::Chunk, cx: i32, cz: i32) -> i32 {
    let air = world.registry().air_id();
    for y in (MIN_Y..=LAST_Y).rev() {
        for i in 0..256 {
            let (x, z) = (cx * 16 + (i & 15), cz * 16 + (i >> 4));
            if chunk.block(x, y, z) != air || chunk.liquid(x, y, z) != air {
                return (y >> 4 << 4) + 15;
            }
        }
    }
    MIN_Y - 1
}

#[test]
#[ignore = "needs worlds/oracle; slow"]
fn shape_matches_bds_near_origin() {
    compare(20261003, "20261003", &[Region::new("chunks -16..15", -16, -16, 15, 15)]);
}

#[test]
#[ignore = "needs worlds/oracle; slow"]
fn shape_matches_bds_far_out() {
    compare(20261003, "20261003", &[Region::new("chunks 100000..100007", 100000, 100000, 100007, 100007)]);
}

#[test]
#[ignore = "needs worlds/oracle; slow"]
fn shape_matches_bds_negative_seed() {
    compare(-4172144997902289642, "-4172144997902289642", &[Region::new("chunks -4..3", -4, -4, 3, 3)]);
}

#[test]
#[ignore = "needs worlds/oracle"]
fn one_chunk_smoke() {
    compare(20261003, "20261003", &[Region::new("chunk 0,0", 0, 0, 0, 0)]);
}
