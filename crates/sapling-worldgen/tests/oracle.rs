//! Exact blocks, carvers and biomes against BDS-saved oracle worlds (tools/worldgen-oracle/README.md).
//! Needs the git-ignored oracle worlds (`$SAPLING_ORACLE_WORLDS`); run with `--ignored --nocapture`.
//! Cells where the world and its `-run2` twin disagree are counted apart, and cells that stages
//! sapling does not run yet may change are reported per stage, not as mismatches.
//! `SAPLING_ORACLE_REGION=<substring>` keeps only the regions whose name contains it (`=<name>`: that region alone);
//! `SAPLING_ORACLE_ORDER=<file>` decorates in a live run's order (docs/research/worldgen-bds-debugger.md).

mod oracle_support;

use std::collections::BTreeMap;

use oracle_support::{
    BdsCell, BiomeTally, CarverTally, Context, Dump, FeatureTally, Floor, Ours, PORTED, Region, Stages, StructureTally, Tally, TopTally, attribute, bds_cell,
    column_reshaped, near_feature, open,
};
use sapling_leveldb::ChunkKey;

use sapling_worldgen::structure::StructureKind;
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
    floor: Floor,
    /// Our highest non-air cell minus BDS's highest terrain cell, per column.
    top_offsets: BTreeMap<i32, (u64, (i32, i32))>,
}

fn compare(seed: i64, world: &str, regions: &[Region]) {
    // The world's twin pair saved by a BDS that ran no scheduled tick in game time (scripts/bds_still.py).
    let still = std::env::var_os("SAPLING_ORACLE_STILL").is_some();
    let world = &if still { format!("{world}-still") } else { world.to_string() };
    let Some((primary, twin)) = open(world) else {
        eprintln!("oracle world {world} missing; skipped");
        return;
    };
    let mut generator = OverworldGenerator::new(seed);
    generator.track_reach();
    // The decoration order of the live run that saved the world (scripts/dbg_order.py), `x z` per line.
    oracle_support::replay(&mut generator);
    let generator = generator;
    let only = std::env::var("SAPLING_ORACLE_REGION").ok();
    let kept = |name: &str| only.as_deref().is_none_or(|only| only.strip_prefix('=').map_or(name.contains(only), |exact| name == exact));
    for region in regions.iter().filter(|r| kept(r.name)) {
        let mut tallies = Tallies::default();
        let name = &if still { format!("{} (still)", region.name) } else { region.name.to_string() };
        let mut dump = Dump::open(world, name);
        let chunks: Vec<_> = region.chunks().collect();
        Stages::in_order(&generator, &chunks, |(cx, cz), Stages { surfaced, loaded, structured, laked, ours, areas, biomes }| {
            let key = ChunkKey::new(cx, cz, 0);
            let a = primary.load_column(key).unwrap().expect("chunk saved").chunk;
            let b = twin.load_column(key).unwrap().expect("twin chunk saved").chunk;
            tallies.structures.note_areas(&areas);
            // Our structure stage writes only the ported kinds' blocks.
            let ported: Vec<_> = areas.iter().filter(|(kind, _, _)| PORTED.contains(kind)).copied().collect();
            let biome_top = highest_section_top(&primary, &a, cx, cz);
            let (reach, tries) = generator.thread_local_reach(cx, cz);
            tallies.floor.note_tries(tries);
            for z in 0..16 {
                for x in 0..16 {
                    let (wx, wz) = (cx * 16 + x as i32, cz * 16 + z as i32);
                    for y in MIN_Y..=biome_top {
                        tallies.biomes.add(biomes.biome(x, y, z), a.biome(wx, y, wz), (wx, y, wz));
                    }
                    let column: Vec<(i32, BdsCell)> = (MIN_Y..=LAST_Y).rev().map(|y| (y, bds_cell(&primary, &a, wx, y, wz))).collect();
                    let reshaped = column_reshaped(|y| ours.block(x, y, z).is_solid(), &column);
                    let our_top = (MIN_Y..=LAST_Y).rev().find(|&y| ours.block(x, y, z).is_solid());
                    let bds_feature_top = column.iter().find(|(_, c)| c.kind() == CellKind::Solid && !c.is_terrain()).map(|(y, _)| *y);
                    let (mut our_highest, mut bds_highest) = (None, None);
                    for ((y, bds), reshaped) in column.iter().zip(reshaped) {
                        let (y, at) = (*y, (wx, *y, wz));
                        let twin_cell = bds_cell(&twin, &b, wx, y, wz);
                        let stable = *bds == twin_cell;
                        let block = ours.block(x, y, z);
                        let before = surfaced.block(x, y, z);
                        let bds_emptied = bds.is_empty() || bds.is_fluid();
                        let context = Context {
                            carved: before.is_solid() && !loaded.block(x, y, z).is_solid(),
                            structured: loaded.block(x, y, z) != structured.block(x, y, z),
                            laked: structured.block(x, y, z) != laked.block(x, y, z),
                            featured: laked.block(x, y, z) != block,
                            feature_emptied: stable && block.is_solid() && bds_emptied && near_feature(&primary, &a, at),
                            reshaped,
                            bds_feature_above: bds_feature_top.is_some_and(|top| top > y),
                        };
                        tallies.blocks.add(block, bds, stable, at, context);
                        tallies.floor.add(block, bds, &twin_cell, at, context, reach.as_ref());
                        dump.cell(at, laked.block(x, y, z), block, (bds, &twin_cell), context, &areas, PORTED, (&primary, &a));
                        if stable {
                            tallies.features.add(laked.block(x, y, z), block, bds);
                            let in_mineshaft = areas.iter().any(|(kind, _, area)| *kind == StructureKind::Mineshaft && area.contains(wx, y, wz));
                            tallies.structures.add((loaded.block(x, y, z), structured.block(x, y, z), block), bds, attribute(&ported, at), attribute(&areas, at), in_mineshaft);
                            if Some(y) == our_top {
                                tallies.top.add(block, bds, at, context);
                            }
                            let next_to_cave = [y - 1, y + 1].iter().any(|&ny| (MIN_Y..=LAST_Y).contains(&ny) && !surfaced.block(x, ny, z).is_solid());
                            let iceberg = matches!(before, Block::PackedIce | Block::Snow);
                            // Kelp and seagrass stand in water, as BDS's cells with a water layer count.
                            let after = if block.is_waterlogged() { CellKind::Water } else { ours.kind(x, y, z) };
                            let cell = Ours { before: surfaced.kind(x, y, z), after, next_to_cave, iceberg };
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
        });
        // Tests run in parallel: a region's lines stay together.
        let _whole = std::io::stdout().lock();
        println!("seed {seed} {name}:");
        tallies.blocks.print();
        tallies.floor.print(world, name);
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

/// 4x4-chunk regions of the tree biomes the biome regions lack (examples/find_biomes.rs).
#[test]
#[ignore = "needs worlds/oracle; slow"]
fn blocks_match_bds_trees() {
    let regions = [
        Region::new("plains", -62, -46, -59, -43),
        Region::new("forest", 186, 506, 189, 509),
        Region::new("taiga", -382, 130, -379, 133),
        Region::new("birch forest", -126, 42, -123, 45),
        Region::new("dark forest", -590, 410, -587, 413),
        Region::new("snowy taiga", 322, 402, 325, 405),
        Region::new("jungle", 498, -30, 501, -27),
        Region::new("sparse jungle", 82, -630, 85, -627),
        Region::new("bamboo jungle", 170, 346, 173, 349),
        Region::new("savanna", 90, -502, 93, -499),
        Region::new("flower forest", -190, 626, -187, 629),
        Region::new("old growth birch forest", -310, 282, -307, 285),
        Region::new("meadow", 370, -510, 373, -507),
        Region::new("cherry grove", 346, -454, 349, -451),
        Region::new("pale garden", -470, -286, -467, -283),
        Region::new("old growth spruce taiga", -534, -30, -531, -27),
        Region::new("windswept forest", -230, -190, -227, -187),
        Region::new("snowy plains", 570, 402, 573, 405),
        Region::new("sunflower plains", -310, 162, -307, 165),
        Region::new("savanna plateau", 562, -390, 565, -387),
    ];
    compare(20261003, "20261003-trees", &regions);
}

/// Regions around one start of each kind (`/locate` answers from near the origin).
#[test]
#[ignore = "needs worlds/oracle; slow"]
fn blocks_match_bds_structures() {
    let regions = [
        Region::new("village", -66, 4, -59, 11),
        Region::new("igloo, snowy slopes", 40, -29, 44, -25),
        Region::new("igloo, snowy plains", -58, -179, -54, -175),
        Region::new("jungle temple", 79, 4, 83, 8),
        Region::new("desert pyramid", -385, -222, -381, -218),
        Region::new("swamp hut", 238, -116, 242, -112),
        Region::new("shipwreck", -9, -26, -5, -22),
        Region::new("monument", -27, -26, -19, -18),
        Region::new("ruined portal", 9, 16, 13, 20),
        Region::new("trial chambers", 2, 14, 12, 24),
        Region::new("trail ruins", -58, 20, -52, 26),
        Region::new("ancient city", -74, 75, -66, 83),
        Region::new("pillager outpost", 115, -73, 121, -67),
        Region::new("stronghold", -93, -126, -85, -118),
        Region::new("mansion", -516, 992, -508, 1000),
        Region::new("ocean ruins", 18, -40, 22, -36),
    ];
    compare(20261003, "20261003-structures", &regions);
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
