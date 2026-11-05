//! Spawn areas against the tag 0x77 records BDS 1.26 saved for seed 20261003
//! (docs/research/structure-spawns.md): whole box first, then the pieces meeting the chunk.

use crate::terrain::ChunkShape;
use crate::{NetherGenerator, OverworldGenerator};

const SEED: i64 = 20261003;

fn areas(shape: &ChunkShape, structure: &str, y_offset: Option<i32>) -> Vec<[i32; 6]> {
    let areas = shape.spawn_areas();
    assert!(areas.iter().all(|a| a.structure == structure && a.y_offset == y_offset), "{areas:?}");
    assert!(areas.first().is_some_and(|a| a.whole) && areas.iter().skip(1).all(|a| !a.whole), "{areas:?}");
    areas.iter().map(|a| a.bounds).collect()
}

fn overworld(chunk: (i32, i32), structure: &str, y_offset: Option<i32>) -> Vec<[i32; 6]> {
    areas(&OverworldGenerator::new(SEED).chunk_blocks(chunk.0, chunk.1), structure, y_offset)
}

#[test]
fn swamp_hut_keeps_its_start_box_and_its_placed_piece() {
    assert_eq!(overworld((240, -114), "minecraft:swamp_hut", Some(-3)), [[3840, 64, -1824, 3846, 70, -1816], [3840, 70, -1824, 3846, 76, -1816]]);
}

#[test]
fn monument_files_its_building_twice() {
    assert_eq!(overworld((-24, -22), "minecraft:monument", Some(0)), [[-384, 39, -352, -369, 61, -337]; 2]);
}

#[test]
fn outpost_pieces_stand_at_their_heights() {
    // Chunks after the tower's start chunk find their own height, which depends on the run's
    // post-process order (the saved world's was not the canonical one).
    let tower = [1873, 128, -1135, 1887, 149, -1121];
    assert_eq!(overworld((117, -71), "minecraft:pillager_outpost", Some(-3)), [[1872, 63, -1136, 1887, 319, -1121], tower, tower]);
    assert_eq!(overworld((116, -72), "minecraft:pillager_outpost", Some(-3)), [[1865, 63, -1152, 1871, 319, -1137], [1866, 134, -1147, 1871, 139, -1141]]);
}

/// research/captures/bds-jxoutpost.txt's world: columns only the whole box meets keep it alone.
#[test]
fn outpost_columns_no_piece_meets_keep_the_whole_box() {
    assert_eq!(overworld((116, -73), "minecraft:pillager_outpost", Some(-3)), [[1865, 63, -1153, 1871, 319, -1153]]);
    assert_eq!(overworld((117, -73), "minecraft:pillager_outpost", Some(-3)), [[1872, 63, -1153, 1887, 319, -1153]]);
}

#[test]
fn fortress_files_the_pieces_meeting_the_chunk() {
    let generator = NetherGenerator::new(SEED);
    let expected = [[208, 48, 64, 223, 83, 79], [223, 62, 64, 223, 71, 64], [210, 62, 64, 222, 75, 68], [208, 58, 64, 209, 71, 64], [208, 58, 65, 209, 64, 69]];
    assert_eq!(areas(&generator.chunk_blocks(13, 4), "minecraft:fortress", Some(0)), expected);
    assert_eq!(areas(&generator.chunk_blocks(16, 1), "minecraft:fortress", Some(0)), [[256, 48, 16, 271, 83, 31]]);
}

/// The oracle world ~/oracle-worlds/20261003-structures, read with `records <world> tag=119`.
#[test]
fn igloos_file_the_tops_box_at_64_whole_and_on_the_ground() {
    assert_eq!(overworld((42, -27), "minecraft:igloo", Some(0)), [[672, 64, -432, 678, 68, -425], [672, 101, -432, 678, 105, -425]]);
    assert_eq!(overworld((-56, -177), "minecraft:igloo", Some(0)), [[-896, 64, -2832, -890, 68, -2825], [-896, 79, -2832, -890, 83, -2825]]);
}

#[test]
fn desert_pyramids_file_the_start_box_and_the_placed_one() {
    assert_eq!(overworld((-383, -220), "minecraft:desert_pyramid", Some(0)), [[-6128, 64, -3520, -6113, 78, -3505], [-6128, 63, -3520, -6113, 77, -3505]]);
    assert_eq!(overworld((-382, -219), "minecraft:desert_pyramid", Some(0)), [[-6112, 64, -3504, -6108, 78, -3500], [-6112, 63, -3504, -6108, 77, -3500]]);
}

#[test]
fn camps_file_their_variant_with_the_whole_box_in_chunks_no_piece_meets() {
    let camp = "minecraft:abandoned_camp_meadow";
    assert_eq!(overworld((114, -67), camp, None), [[1835, 116, -1064, 1839, 123, -1057], [1835, 116, -1063, 1839, 123, -1057]]);
    assert_eq!(overworld((114, -66), camp, None), [[1835, 116, -1056, 1839, 123, -1055], [1835, 116, -1056, 1839, 123, -1056]]);
    assert_eq!(overworld((115, -67), camp, None), [[1840, 116, -1064, 1850, 123, -1057], [1840, 116, -1063, 1840, 123, -1057], [1841, 116, -1064, 1850, 118, -1057]]);
    let birch = "minecraft:abandoned_camp_birch_forest";
    assert_eq!(overworld((15, 20), birch, None), [[246, 80, 320, 255, 87, 328], [246, 80, 320, 255, 82, 328]]);
}

#[test]
fn trial_chambers_file_their_whole_box_alone_where_no_piece_reaches() {
    let chambers = "minecraft:trial_chambers";
    assert_eq!(overworld((3, 17), chambers, None), [[48, -45, 272, 63, 15, 287]]);
    assert_eq!(overworld((3, 21), chambers, None), [[48, -45, 336, 63, 15, 351], [48, -4, 343, 63, 15, 351], [63, 8, 346, 63, 10, 348]]);
}

#[test]
fn trail_ruins_file_each_piece() {
    let ruins = "minecraft:trail_ruins";
    assert_eq!(overworld((-55, 21), ruins, None), [[-876, 79, 345, -865, 102, 351], [-868, 80, 350, -865, 84, 351], [-868, 79, 345, -865, 83, 349]]);
}

#[test]
fn data_driven_areas_come_before_hardcoded_ones() {
    let shape = OverworldGenerator::new(SEED).chunk_blocks(81, 6);
    let got: Vec<(&str, [i32; 6], bool)> = shape.spawn_areas().iter().map(|a| (a.structure, a.bounds, a.whole)).collect();
    let (chambers, jungle) = ("minecraft:trial_chambers", "minecraft:jungle_pyramid");
    let expected = [
        (chambers, [1296, -44, 96, 1311, -11, 111], true),
        (chambers, [1296, -41, 96, 1311, -23, 96], false),
        (chambers, [1296, -30, 97, 1299, -26, 97], false),
        (chambers, [1306, -39, 96, 1308, -38, 96], false),
        (chambers, [1296, -38, 96, 1298, -35, 96], false),
        (chambers, [1301, -39, 96, 1303, -38, 96], false),
        (jungle, [1296, 64, 96, 1307, 73, 110], true),
        (jungle, [1296, 72, 96, 1307, 81, 110], false),
    ];
    assert_eq!(got, expected);
}
