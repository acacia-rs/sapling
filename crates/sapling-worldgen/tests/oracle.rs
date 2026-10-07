//! Exact blocks, carvers and biomes against BDS-saved oracle worlds (tools/worldgen-oracle/README.md).
//! Needs the git-ignored oracle worlds (`$SAPLING_ORACLE_WORLDS`); run with `--ignored --nocapture`.
//! Cells where the world and its `-run2` twin disagree are counted apart, and cells that stages
//! sapling does not run yet may change are reported per stage, not as mismatches.
//! `SAPLING_ORACLE_REGION=<substring>` keeps only the regions whose name contains it.

mod oracle_support;

use std::collections::BTreeMap;

use oracle_support::{
    BdsCell, BiomeTally, CarverTally, Context, FeatureTally, Ours, Region, StructureTally, Tally, TopTally, attribute, bds_cell, column_reshaped,
    near_feature, open,
};
use sapling_leveldb::ChunkKey;

use sapling_worldgen::{Block, CellKind, MIN_Y, OverworldGenerator};

const LAST_Y: i32 = 319;

#[derive(Default)]
struct Tallies {
    blocks: Tally,
    top: TopTally,
    biomes: BiomeTally,
    carvers: CarverTally,
    features: FeatureTally,
    structures: StructureTally,
    /// Our highest non-air cell minus BDS's highest terrain cell, per column.
    top_offsets: BTreeMap<i32, (u64, (i32, i32))>,
}

fn compare(seed: i64, world: &str, regions: &[Region]) {
    let Some((primary, twin)) = open(world) else {
        eprintln!("oracle world {world} missing; skipped");
        return;
    };
    let generator = OverworldGenerator::new(seed);
    let only = std::env::var("SAPLING_ORACLE_REGION").ok();
    for region in regions.iter().filter(|r| only.as_deref().is_none_or(|only| r.name.contains(only))) {
        let mut tallies = Tallies::default();
        for (cx, cz) in region.chunks() {
            let key = ChunkKey::new(cx, cz, 0);
            let a = primary.load_column(key).unwrap().expect("chunk saved").chunk;
            let b = twin.load_column(key).unwrap().expect("twin chunk saved").chunk;
            let surfaced = generator.surfaced_blocks(cx, cz);
            let mut loaded = surfaced.clone();
            generator.carve(&mut loaded, cx, cz);
            let mut structured = loaded.clone();
            generator.place_structures(&mut structured, cx, cz);
            let areas = generator.structure_areas(cx, cz);
            tallies.structures.note_areas(&areas);
            let mut laked = structured.clone();
            generator.decorate(&mut laked, cx, cz);
            let mut ours = laked.clone();
            generator.place_features(&mut ours, cx, cz);
            let biomes = generator.chunk_biomes(cx, cz);
            let biome_top = highest_section_top(&primary, &a, cx, cz);
            for z in 0..16 {
                for x in 0..16 {
                    let (wx, wz) = (cx * 16 + x as i32, cz * 16 + z as i32);
                    for y in MIN_Y..=biome_top {
                        tallies.biomes.add(biomes.biome(x, y, z), a.biome(wx, y, wz), (wx, y, wz));
                    }
                    let column: Vec<(i32, BdsCell)> = (MIN_Y..=LAST_Y).rev().map(|y| (y, bds_cell(&primary, &a, wx, y, wz))).collect();
                    let reshaped = column_reshaped(|y| ours.block(x, y, z).is_solid(), &column);
                    let our_top = (MIN_Y..=LAST_Y).rev().find(|&y| ours.block(x, y, z).is_solid());
                    let (mut our_highest, mut bds_highest) = (None, None);
                    for ((y, bds), reshaped) in column.iter().zip(reshaped) {
                        let (y, at) = (*y, (wx, *y, wz));
                        let stable = *bds == bds_cell(&twin, &b, wx, y, wz);
                        let block = ours.block(x, y, z);
                        let before = surfaced.block(x, y, z);
                        let bds_emptied = bds.is_empty() || bds.is_fluid();
                        let context = Context {
                            carved: before.is_solid() && !loaded.block(x, y, z).is_solid(),
                            laked: structured.block(x, y, z) != laked.block(x, y, z),
                            featured: laked.block(x, y, z) != block,
                            feature_emptied: stable && block.is_solid() && bds_emptied && near_feature(&primary, &a, at),
                            reshaped,
                        };
                        tallies.blocks.add(block, bds, stable, at, context);
                        if stable {
                            tallies.features.add(laked.block(x, y, z), block, bds);
                            tallies.structures.add(loaded.block(x, y, z), structured.block(x, y, z), bds, attribute(&areas, at));
                            if Some(y) == our_top {
                                tallies.top.add(block, bds, at, context);
                            }
                            let next_to_cave = [y - 1, y + 1].iter().any(|&ny| (MIN_Y..=LAST_Y).contains(&ny) && !surfaced.block(x, ny, z).is_solid());
                            let iceberg = matches!(before, Block::PackedIce | Block::Snow);
                            let cell = Ours { before: surfaced.kind(x, y, z), after: ours.kind(x, y, z), next_to_cave, iceberg };
                            tallies.carvers.add(cell, bds, at, &primary, &a);
                        }
                        if our_highest.is_none() && ours.kind(x, y, z) != CellKind::Air {
                            our_highest = Some(y);
                        }
                        if bds_highest.is_none() && bds.is_terrain() {
                            bds_highest = Some(y);
                        }
                    }
                    let offset = our_highest.unwrap_or(0) - bds_highest.unwrap_or(0);
                    tallies.top_offsets.entry(offset).or_insert((0, (wx, wz))).0 += 1;
                }
            }
        }
        println!("seed {seed} {}:", region.name);
        tallies.blocks.print();
        tallies.top.print();
        tallies.features.print();
        tallies.structures.print();
        tallies.carvers.print();
        tallies.biomes.print();
        let offsets: Vec<String> = tallies.top_offsets.iter().map(|(d, (n, at))| format!("{d:+}:{n} {at:?}")).collect();
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

/// 4x4-chunk regions picked by examples/find_biomes.rs, one per surface builder or material case.
#[test]
#[ignore = "needs worlds/oracle; slow"]
fn blocks_match_bds_biomes() {
    let regions = [
        Region::new("badlands", 594, -350, 597, -347),
        Region::new("eroded badlands", 170, -598, 173, -595),
        Region::new("wooded badlands", 298, -558, 301, -555),
        Region::new("swamp", 314, 306, 317, 309),
        Region::new("mangrove swamp", -502, 514, -499, 517),
        Region::new("frozen ocean", 474, 570, 477, 573),
        Region::new("deep frozen ocean", 450, 458, 453, 461),
        Region::new("jagged peaks", -406, -382, -403, -379),
        Region::new("frozen peaks", 442, 306, 445, 309),
        Region::new("stony peaks", -230, 258, -227, 261),
        Region::new("snowy slopes", 282, 66, 285, 69),
        Region::new("grove", -446, 66, -443, 69),
        Region::new("windswept hills", 226, -102, 229, -99),
        Region::new("windswept gravelly hills", -126, -350, -123, -347),
        Region::new("old growth pine taiga", -334, 338, -331, 341),
        Region::new("windswept savanna", -590, -214, -587, -211),
        Region::new("desert", -446, 554, -443, 557),
        Region::new("ice spikes", -22, -198, -19, -195),
        Region::new("mushroom fields", -46, 354, -43, 357),
        Region::new("warm ocean", 242, -494, 245, -491),
    ];
    compare(20261003, "20261003-biomes", &regions);
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
