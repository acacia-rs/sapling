//! `OverworldGenerator::place_structure` and `place_jigsaw` over a flat stone world.

use std::sync::Arc;

use super::jigsaw::data;
use super::{PlaceFailure, StructureKind};
use crate::block::Block;
use crate::block_entity::BlockEntity;
use crate::terrain::{ChunkShape, MIN_Y};
use crate::OverworldGenerator;

const SEED: i64 = 20261003;

/// Stone up to y 64, air above.
fn flat(_: i32, _: i32) -> Arc<ChunkShape> {
    let mut shape = ChunkShape::empty();
    for y in MIN_Y..=64 {
        for (x, z) in (0..16).flat_map(|x| (0..16).map(move |z| (x, z))) {
            shape.set_block(x, y, z, Block::Stone);
        }
    }
    Arc::new(shape)
}

#[test]
fn place_jigsaw_draws_a_trail_ruins_tower() {
    let generator = OverworldGenerator::new(SEED);
    let placed = generator.place_jigsaw("minecraft:trail_ruins/tower", "minecraft:road_anchor", 7, [8, 40, 8], &flat);
    let names: Vec<String> = data().pool("trail_ruins/tower").unwrap().list.iter().flat_map(|e| e.jigsaws((0, 0, 0), super::template::Rotation::None)).map(|j| j.name.to_string()).collect();
    let writes = placed.unwrap_or_else(|e| panic!("{e:?}; the pool's jigsaws are named {names:?}"));
    assert!(!writes.blocks.is_empty());
}

#[test]
fn place_jigsaw_without_the_namespace_matches() {
    let generator = OverworldGenerator::new(SEED);
    let full = generator.place_jigsaw("minecraft:trail_ruins/tower", "minecraft:road_anchor", 2, [8, 40, 8], &flat);
    assert_eq!(generator.place_jigsaw("trail_ruins/tower", "road_anchor", 2, [8, 40, 8], &flat), full);
    assert!(full.is_ok_and(|writes| !writes.blocks.is_empty()));
}

#[test]
fn place_jigsaw_rejects_an_unknown_pool_or_target() {
    let generator = OverworldGenerator::new(SEED);
    assert_eq!(generator.place_jigsaw("minecraft:no_such/pool", "minecraft:road_anchor", 7, [8, 40, 8], &flat), Err(PlaceFailure::Unknown));
    assert_eq!(generator.place_jigsaw("minecraft:trail_ruins/tower", "minecraft:no_such_jigsaw", 7, [8, 40, 8], &flat), Err(PlaceFailure::Failed));
}

#[test]
fn place_jigsaw_joins_a_tower_only_at_its_road_anchor() {
    // research/captures/bds-ai1camps.txt: `road_anchor` placed, `bottom` and an unknown target a generation error.
    let generator = OverworldGenerator::new(SEED);
    let at = |target: &str| generator.place_jigsaw("minecraft:trail_ruins/tower", target, 3, [8, 40, 8], &flat).map(|_| ());
    assert_eq!([at("minecraft:road_anchor"), at("minecraft:bottom"), at("minecraft:no_such_target")], [Ok(()), Err(PlaceFailure::Failed), Err(PlaceFailure::Failed)]);
}

#[test]
fn place_structure_draws_trail_ruins_and_a_village() {
    let generator = OverworldGenerator::new(SEED);
    for (kind, chunk) in [(StructureKind::TrailRuins, (0, 0)), (StructureKind::Village, (-62, 8))] {
        let writes = generator.place_structure(kind, chunk, &flat).unwrap_or_else(|e| panic!("{kind:?}: {e:?}"));
        assert!(!writes.blocks.is_empty(), "{kind:?}");
    }
}

#[test]
fn placed_structures_carry_their_loot() {
    let generator = OverworldGenerator::new(SEED);
    // Over the generator's own terrain: a village floating over the flat world places no houses.
    let carved = |x: i32, z: i32| generator.carved(x, z);
    let tables = |kind, chunk| {
        let writes = generator.place_structure(kind, chunk, &carved).unwrap();
        let block = |at: [i32; 3]| writes.blocks.iter().rev().find(|(p, _)| *p == at).map(|(_, b)| *b);
        for (at, entity) in &writes.entities {
            assert!(block(*at).is_some_and(|b| entity.fits(b.name())), "{kind:?}: {entity:?} at {at:?}");
        }
        writes.entities.iter().filter_map(|(_, e)| if let BlockEntity::Loot { table, .. } = e { Some(*table) } else { None }).collect::<Vec<_>>()
    };
    assert!(tables(StructureKind::TrailRuins, (0, 0)).iter().any(|t| t.contains("trail_ruins_brushable_block")));
    let village = tables(StructureKind::Village, (-62, 8));
    assert!(village.iter().any(|t| t.starts_with("loot_tables/chests/village/")), "{village:?}");
}

#[test]
fn a_placed_village_names_its_marker_mobs() {
    let generator = OverworldGenerator::new(SEED);
    let carved = |x: i32, z: i32| generator.carved(x, z);
    let writes = generator.place_structure(StructureKind::Village, (-62, 8), &carved).unwrap();
    let kinds: Vec<&str> = writes.actors.iter().map(|actor| actor.kind).collect();
    // bds-k1jig.txt: BDS's `/place jigsaw` of a town centre made its golem.
    assert!(kinds.contains(&"minecraft:iron_golem") && kinds.contains(&"minecraft:villager_v2"), "{kinds:?}");
}

#[test]
fn a_placed_town_centre_files_the_jigsaws_keep_jigsaws_keeps() {
    // bds-k1jig.txt: BDS kept a town centre's jigsaws as JigsawBlocks, up ones facing 1, sides 2-5.
    let generator = OverworldGenerator::new(SEED);
    let town = generator.place_jigsaw("minecraft:village/plains/town_centers", "minecraft:bottom", 1, [8, 65, 8], &flat).unwrap();
    let writes = generator.place_jigsaw("minecraft:trail_ruins/tower", "minecraft:road_anchor", 7, [8, 40, 8], &flat).unwrap();
    assert!(!writes.jigsaws.is_empty() && writes.entities.iter().all(|(_, entity)| !matches!(entity, BlockEntity::Jigsaw { .. })), "{} blocks; town {} {}", writes.blocks.len(), town.blocks.len(), town.jigsaws.len());
    for (at, jigsaw) in &writes.jigsaws {
        let BlockEntity::Jigsaw { facing, name, .. } = jigsaw else { panic!("{jigsaw:?} at {at:?}") };
        assert!(*facing < 6 && name.starts_with("minecraft:"), "{jigsaw:?} at {at:?}");
    }
}

#[test]
fn ignore_start_height_stands_trail_ruins_at_the_given_height() {
    // bds-n13jig.txt: from y -50 and -40 BDS's ruins topped out at -35 and -25, from their own height buried at the floor.
    let generator = OverworldGenerator::new(SEED);
    let top = |start_y| generator.place_structure_at(StructureKind::TrailRuins, (0, 0), start_y, &flat).unwrap().blocks.iter().map(|([_, y, _], _)| *y).max();
    assert_eq!(top(Some(-20)), Some(-5));
    assert_ne!(top(None), top(Some(-20)));
}

#[test]
fn the_place_command_starts_trail_ruins_at_its_block_with_that_block_s_random() {
    // PlaceCommand 143aed7f0: Java's random forked at the block, the start piece's corner there; two
    // blocks of one chunk start different ruins, the same block the same.
    let generator = OverworldGenerator::new(SEED);
    let placed = |at| generator.place_commanded("minecraft:trail_ruins", at, true, &flat).unwrap().blocks;
    let [a, b] = [[70, -20, 210], [75, -20, 213]].map(placed);
    assert_ne!(a, b);
    assert_eq!(a, placed([70, -20, 210]));
    assert_eq!(generator.place_commanded("minecraft:village", [0, 0, 0], true, &flat).err(), Some(PlaceFailure::Unknown));
}

#[test]
fn the_place_command_seeds_each_piece_s_loot_from_its_column_s_random() {
    // research/captures/bds-adplace-chests.txt: every piece's share of a chunk draws anew from Java's
    // random forked at the command's (x, 0, z), its second container taking the second draw.
    let generator = OverworldGenerator::new(SEED);
    let carved = |x: i32, z: i32| generator.carved(x, z);
    let seeds = |name, at, keep_height| {
        let writes = generator.place_commanded(name, at, keep_height, &carved).unwrap();
        writes.entities.into_iter().filter_map(|(pos, e)| if let BlockEntity::Loot { seed, .. } = e { Some((pos, seed)) } else { None }).collect::<Vec<_>>()
    };
    let camp = |at| seeds("minecraft:abandoned_camp_forest", at, false);
    assert_eq!(camp([10, 70, 205]), [([15, 72, 203], -1105909668), ([12, 71, 212], -1105909668), ([12, 72, 212], 744381607)]);
    assert_eq!(camp([30, 70, 195]), [([26, 72, 193], 1057644916), ([33, 70, 191], 1057644916), ([35, 72, 192], 1057644916)]);
    // Trail ruins fork at their start height: drawn (-15) without ignoreStartHeight, the block's with it.
    assert!(seeds("minecraft:trail_ruins", [-10, 70, 210], false).contains(&([1, 57, 209], -1408014870)));
    assert!(seeds("minecraft:trail_ruins", [70, 70, 210], true).contains(&([44, 68, 205], 179796147)));
}

#[test]
fn place_structure_fails_a_temple_in_the_ocean() {
    // The monument BDS located (structure/tests.rs): deep ocean, no temple kind there.
    let generator = OverworldGenerator::new(SEED);
    assert_eq!(generator.place_structure(StructureKind::Temple, (-23, -22), &flat), Err(PlaceFailure::Failed));
}
