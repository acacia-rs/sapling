//! Exact blocks and biomes against BDS-saved oracle worlds (tools/worldgen-oracle/README.md).
//! Needs the git-ignored oracle worlds (`WORLDGEN_ORACLE_DIR`); run with `--ignored --nocapture`.
//! Cells where the world and its `-run2` twin disagree are counted apart, and cells that stages
//! sapling does not run yet may change are reported per stage, not as mismatches.

mod oracle_support;

use oracle_support::{BdsCell, BiomeTally, Region, Tally, TopTally, bds_cell, open, terrain_top};
use sapling_leveldb::ChunkKey;

use sapling_worldgen::{MIN_Y, OverworldGenerator};

const LAST_Y: i32 = 319;

fn compare(seed: i64, world: &str, regions: &[Region]) {
    let Some((primary, twin)) = open(world) else {
        eprintln!("oracle world {world} missing; skipped");
        return;
    };
    let generator = OverworldGenerator::new(seed);
    for region in regions {
        let mut tally = Tally::default();
        let mut top = TopTally::default();
        let mut biome_tally = BiomeTally::default();
        for (cx, cz) in region.chunks() {
            let key = ChunkKey::new(cx, cz, 0);
            let a = primary.load_column(key).unwrap().expect("chunk saved").chunk;
            let b = twin.load_column(key).unwrap().expect("twin chunk saved").chunk;
            let ours = generator.chunk_blocks(cx, cz);
            let biomes = generator.chunk_biomes(cx, cz);
            let biome_top = highest_section_top(&primary, &a, cx, cz);
            for z in 0..16 {
                for x in 0..16 {
                    let (wx, wz) = (cx * 16 + x as i32, cz * 16 + z as i32);
                    for y in MIN_Y..=biome_top {
                        biome_tally.add(biomes.biome(x, y, z), a.biome(wx, y, wz), (wx, y, wz));
                    }
                    let column: Vec<(i32, BdsCell, bool)> = (MIN_Y..=LAST_Y)
                        .rev()
                        .map(|y| {
                            let bds = bds_cell(&primary, &a, wx, y, wz);
                            let stable = bds == bds_cell(&twin, &b, wx, y, wz);
                            (y, bds, stable)
                        })
                        .collect();
                    let our_top = (MIN_Y..=LAST_Y).rev().find(|&y| ours.block(x, y, z).is_solid());
                    let reshaped = our_top != terrain_top(column.iter().map(|(y, cell, _)| (*y, cell)));
                    for (y, bds, stable) in &column {
                        let block = ours.block(x, *y, z);
                        tally.add(block, bds, *stable, (wx, *y, wz), reshaped);
                        if Some(*y) == our_top && *stable {
                            top.add(block, bds, (wx, *y, wz), reshaped);
                        }
                    }
                }
            }
        }
        println!("seed {seed} {}:", region.name);
        tally.print();
        top.print();
        biome_tally.print();
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
fn blocks_match_bds_near_origin() {
    compare(20261003, "20261003", &[Region::new("chunks -16..15", -16, -16, 15, 15)]);
}

#[test]
#[ignore = "needs worlds/oracle; slow"]
fn blocks_match_bds_far_out() {
    compare(20261003, "20261003", &[Region::new("chunks 100000..100007", 100000, 100000, 100007, 100007)]);
}

#[test]
#[ignore = "needs worlds/oracle; slow"]
fn blocks_match_bds_negative_seed() {
    compare(-4172144997902289642, "-4172144997902289642", &[Region::new("chunks -4..3", -4, -4, 3, 3)]);
}

#[test]
#[ignore = "needs worlds/oracle"]
fn one_chunk_smoke() {
    compare(20261003, "20261003", &[Region::new("chunk 0,0", 0, 0, 0, 0)]);
}

#[test]
#[ignore = "needs worlds/oracle"]
fn a_few_chunks() {
    compare(20261003, "20261003", &[Region::new("chunks -2..1", -2, -2, 1, 1)]);
}
