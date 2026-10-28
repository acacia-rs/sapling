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
fn place_structure_fails_a_temple_in_the_ocean() {
    // The monument BDS located (structure/tests.rs): deep ocean, no temple kind there.
    let generator = OverworldGenerator::new(SEED);
    assert_eq!(generator.place_structure(StructureKind::Temple, (-23, -22), &flat), Err(PlaceFailure::Failed));
}
