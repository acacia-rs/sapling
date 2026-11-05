//! Spawn areas against the tag 0x77 records BDS 1.26 saved for seed 20261003
//! (docs/research/structure-spawns.md): whole box first, then the pieces meeting the chunk.

use crate::terrain::ChunkShape;
use crate::{NetherGenerator, OverworldGenerator};

const SEED: i64 = 20261003;

fn areas(shape: &ChunkShape, structure: &str, y_offset: i32) -> Vec<[i32; 6]> {
    let areas = shape.spawn_areas();
    assert!(areas.iter().all(|a| a.structure == structure && a.y_offset == y_offset), "{areas:?}");
    assert!(areas.first().is_some_and(|a| a.whole) && areas.iter().skip(1).all(|a| !a.whole), "{areas:?}");
    areas.iter().map(|a| a.bounds).collect()
}

#[test]
fn swamp_hut_keeps_its_start_box_and_its_placed_piece() {
    let shape = OverworldGenerator::new(SEED).chunk_blocks(240, -114);
    assert_eq!(areas(&shape, "minecraft:swamp_hut", -3), [[3840, 64, -1824, 3846, 70, -1816], [3840, 70, -1824, 3846, 76, -1816]]);
}

#[test]
fn monument_files_its_building_twice() {
    let shape = OverworldGenerator::new(SEED).chunk_blocks(-24, -22);
    assert_eq!(areas(&shape, "minecraft:monument", 0), [[-384, 39, -352, -369, 61, -337]; 2]);
}

#[test]
fn outpost_pieces_stand_at_their_heights() {
    let generator = OverworldGenerator::new(SEED);
    // Chunks after the tower's start chunk find their own height, which depends on the run's
    // post-process order (the saved world's was not the canonical one).
    let tower = [1873, 128, -1135, 1887, 149, -1121];
    assert_eq!(areas(&generator.chunk_blocks(117, -71), "minecraft:pillager_outpost", -3), [[1872, 63, -1136, 1887, 319, -1121], tower, tower]);
    assert_eq!(areas(&generator.chunk_blocks(116, -72), "minecraft:pillager_outpost", -3), [[1865, 63, -1152, 1871, 319, -1137], [1866, 134, -1147, 1871, 139, -1141]]);
}

/// research/captures/bds-jxoutpost.txt's world: columns only the whole box meets keep it alone.
#[test]
fn outpost_columns_no_piece_meets_keep_the_whole_box() {
    let generator = OverworldGenerator::new(SEED);
    assert_eq!(areas(&generator.chunk_blocks(116, -73), "minecraft:pillager_outpost", -3), [[1865, 63, -1153, 1871, 319, -1153]]);
    assert_eq!(areas(&generator.chunk_blocks(117, -73), "minecraft:pillager_outpost", -3), [[1872, 63, -1153, 1887, 319, -1153]]);
}

#[test]
fn fortress_files_the_pieces_meeting_the_chunk() {
    let generator = NetherGenerator::new(SEED);
    let expected = [[208, 48, 64, 223, 83, 79], [223, 62, 64, 223, 71, 64], [210, 62, 64, 222, 75, 68], [208, 58, 64, 209, 71, 64], [208, 58, 65, 209, 64, 69]];
    assert_eq!(areas(&generator.chunk_blocks(13, 4), "minecraft:fortress", 0), expected);
    assert_eq!(areas(&generator.chunk_blocks(16, 1), "minecraft:fortress", 0), [[256, 48, 16, 271, 83, 31]]);
}
