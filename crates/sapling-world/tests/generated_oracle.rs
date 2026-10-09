//! A column generated through `Generated` against the BDS-saved oracle world (sapling-worldgen
//! tests/oracle.rs, tools/worldgen-oracle/README.md). Needs `$SAPLING_ORACLE_WORLDS`; run with
//! `--ignored --nocapture`. Every cell where the generator's block is the one BDS saved, which is
//! what the oracle test counts as a match, must convert to BDS's exact state; the rest are later
//! stages (features, structures, liquid flow) and are only counted.

use std::path::PathBuf;

use acacia_world::BlockRegistry;
use sapling_leveldb::{ChunkKey, LevelDbWorld};
use sapling_world::{ChunkPos, Generated, MemoryStorage, WorldStorage};
use sapling_worldgen::{MIN_Y, OverworldGenerator};

const SEED: i64 = 20261003;

#[test]
#[ignore = "needs the oracle worlds"]
fn generated_column_matches_bds_wherever_the_generator_does() {
    let Some(root) = std::env::var_os("SAPLING_ORACLE_WORLDS").map(PathBuf::from) else {
        eprintln!("SAPLING_ORACLE_WORLDS unset; skipped");
        return;
    };
    let bds = LevelDbWorld::open_in_memory(&root.join(SEED.to_string())).expect("the oracle world opens");
    let theirs = bds.load_column(ChunkKey::new(0, 0, 0)).unwrap().expect("BDS saved chunk 0, 0").chunk;
    let ours = Generated::new(MemoryStorage::default(), SEED).load_column(ChunkPos::new(0, 0), 0).unwrap().expect("generated").chunk;
    let generator = OverworldGenerator::new(SEED);
    let (shape, biomes) = (generator.chunk_blocks(0, 0), generator.chunk_biomes(0, 0));
    let (registry, vanilla) = (bds.registry(), BlockRegistry::vanilla());
    let state = |registry: &BlockRegistry, id| registry.get(id).map(|s| (s.name, s.properties));

    let (mut cells, mut matching, mut biomes_agree) = (0, 0, 0);
    for y in MIN_Y..320 {
        for (x, z) in (0..16).flat_map(|x| (0..16).map(move |z| (x, z))) {
            let (wx, wz) = (x as i32, z as i32);
            cells += 1;
            let generated = shape.block(x, y, z);
            let bds_block = state(registry, theirs.block(wx, y, wz)).unwrap();
            let bds_liquid = theirs.liquid(wx, y, wz);
            let bds_wet = bds_liquid != registry.air_id();
            biomes_agree += usize::from(theirs.biome(wx, y, wz) == Some(biomes.biome(x, y, z)));
            let same = bds_block.0.strip_prefix("minecraft:") == Some(generated.name()) && bds_block.1 == generated.states();
            if !same || bds_wet != generated.second_layer().is_some() {
                continue;
            }
            matching += 1;
            assert_eq!(state(vanilla, ours.block(wx, y, wz)), Some(bds_block), "block at {x} {y} {z}");
            assert_eq!(state(vanilla, ours.liquid(wx, y, wz)), state(registry, bds_liquid), "liquid at {x} {y} {z}");
        }
    }
    println!("chunk 0, 0: {matching} of {cells} cells match BDS and convert to its state; biomes agree in {biomes_agree} (BDS copies its highest sub-chunk's upward)");
    assert!(matching * 10 > cells * 9, "the generator should match most cells");
}
