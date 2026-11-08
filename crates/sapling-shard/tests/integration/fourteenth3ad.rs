//! Group AD of the fourteenth brief: a chest set away while open (research/captures/bds-u3chest.txt)
//! vehicles broken in lava or killed (bds-ffburn.txt, bds-p3lava.txt), and a structure placed over
//! columns not loaded (bds-y3place.txt).

use crate::common;

use acacia_proto::Packet;
use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, AddItemEntity, BlockEvent, ContainerClose, ContainerOpen, UpdateBlock};
use std::sync::Arc;

use common::{Harness, all, click};
use sapling_api::{Block, PlaceOptions};
use sapling_world::{BlockRegistry, Chunk, ChunkPos, Dimension, PlaceFailure, ShapeLoader, StoredColumn, World, WorldStorage};
use sapling_worldgen::StructureWrites;

const HERE: [i32; 3] = [1, 65, 0];

fn alex() -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.tick_all();
    h
}

#[test]
fn a_chest_set_away_while_open_is_shut_before_it_is_shown_gone() {
    // bds-u3chest.txt: lid 0, ContainerClose (server), lid 0, then the cell's UpdateBlock.
    let mut h = alex();
    h.remote.run(|ctx| assert!(ctx.set_block(HERE, &Block::new("chest").with("minecraft:cardinal_direction=north"))));
    h.tick_all();
    h.receive(1, &click(HERE, 1, None));
    assert_eq!(all::<ContainerOpen>(h.tick_all().to(1)).len(), 1);
    h.tick_all();
    h.remote.run(|ctx| assert!(ctx.set_block(HERE, &Block::air())));
    let sent = h.tick_all().to(1).to_vec();
    let told: Vec<u32> = sent.iter().map(|packet| packet.id).filter(|id| [BlockEvent::ID, ContainerClose::ID, UpdateBlock::ID].contains(id)).collect();
    assert_eq!(told, [BlockEvent::ID, ContainerClose::ID, BlockEvent::ID, UpdateBlock::ID]);
}

/// Summons `kind` at `feet` and watches it break: its told height and its item's.
fn told_and_dropped(h: &mut Harness, kind: &'static str, feet: [f32; 3]) -> (f32, f32) {
    h.remote.run(move |ctx| assert!(ctx.summon_as(None, kind, feet, 0.0, 0.0, None).is_some()));
    let sent: Vec<RawPacket> = (0..8).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let added = all::<AddEntity>(&sent).into_iter().find(|added| added.entity_type == kind).expect("summoned");
    let dropped = all::<AddItemEntity>(&sent).first().map(|item| item.position.y).expect("broken into its item");
    (added.position.y, dropped)
}

#[test]
fn a_boat_burnt_in_lava_drops_where_it_was_summoned_and_a_cart_a_fall_further_down() {
    // bds-ffburn.txt: the boat's item 0.125 over where it was shown; the cart's 0.04 + 0.08 + 0.12 lower,
    // the last fall in the tick it burnt through, untold.
    let mut h = alex();
    h.remote.run(|ctx| {
        assert!(ctx.update_block([4, 64, 2], &Block::new("lava")));
        assert!(ctx.update_block([8, 64, 2], &Block::new("lava")));
    });
    h.tick_all();
    let (boat, boat_item) = told_and_dropped(&mut h, "minecraft:boat", [4.5, 65.0, 2.5]);
    assert!((boat_item - boat - 0.125).abs() < 1e-4, "{boat} -> {boat_item}");
    let (cart, cart_item) = told_and_dropped(&mut h, "minecraft:minecart", [8.5, 65.0, 2.5]);
    assert!((cart_item - (cart - 0.24 + 0.125)).abs() < 1e-3, "{cart} -> {cart_item}");
}

#[test]
fn a_killed_boat_leaves_no_item_and_a_killed_cart_does() {
    // bds-ffburn.txt, bds-t2eboats.txt: `/kill` removed boats with nothing, carts with their item.
    let mut h = alex();
    let dropped = |h: &mut Harness, kind: &'static str| {
        let id = common::asking::ask(h, move |ctx| ctx.summon_as(None, kind, [3.5, 65.0, 6.5], 0.0, 0.0, None)).expect("summoned");
        h.remote.run(move |ctx| assert!(ctx.kill_entity(id)));
        (0..3).flat_map(|_| all::<AddItemEntity>(h.tick_all().to(1))).count()
    };
    assert_eq!(dropped(&mut h, "minecraft:boat"), 0);
    assert_eq!(dropped(&mut h, "minecraft:minecart"), 1);
}

const NEAR: [i32; 3] = [8, 64, 8];
const FAR: [i32; 3] = [88, 64, 8];

/// Storage of a stone floor at y 63 whose one structure is a glowstone at NEAR and one at FAR.
struct OneStructure(sapling_world::MemoryStorage);

impl WorldStorage for OneStructure {
    fn load_column(&self, pos: ChunkPos, dimension: i32) -> Result<Option<StoredColumn>, sapling_world::StorageError> {
        if let Some(stored) = self.0.load_column(pos, dimension)? {
            return Ok(Some(stored));
        }
        let registry = BlockRegistry::vanilla();
        let mut chunk = Chunk::empty(pos.x, pos.z, Dimension::overworld(registry.air_id()));
        let stone = registry.states_of("minecraft:stone").next().unwrap().0;
        (0..16).for_each(|x| (0..16).for_each(|z| chunk.set(x, 63, z, 0, stone)));
        Ok(Some(StoredColumn::of(chunk)))
    }

    fn store_column(&self, pos: ChunkPos, dimension: i32, column: &StoredColumn) -> Result<(), sapling_world::StorageError> {
        self.0.store_column(pos, dimension, column)
    }

    fn structure_writes(&self, _: &str, _: [i32; 3], _: bool, _: &ShapeLoader<'_>) -> Option<Result<StructureWrites, PlaceFailure>> {
        let glowstone = sapling_worldgen::Block::from_name("minecraft:glowstone")?;
        Some(Ok(StructureWrites { blocks: vec![(NEAR, glowstone), (FAR, glowstone)], entities: Vec::new(), actors: Vec::new(), jigsaws: Vec::new() }))
    }
}

fn name_at(h: &mut Harness, pos: [i32; 3]) -> String {
    common::asking::ask(h, move |ctx| ctx.block(pos).map(|block| block.name)).unwrap_or_default()
}

#[test]
fn a_placed_structure_s_share_of_an_unloaded_column_is_placed_when_the_column_loads() {
    // BDS's PlaceCommand `_placeOrDeferSections`: success, the loaded part now, the rest as it loads (bds-y3place.txt).
    let world = World::with_storage(Arc::new(OneStructure(sapling_world::MemoryStorage::default())));
    let mut h = Harness::with(common::config(), world);
    common::stored::enter_stored(&mut h, 1);
    let placed = common::asking::ask(&mut h, |ctx| ctx.place_structure("minecraft:trail_ruins", [8, 64, 8], PlaceOptions::default()));
    assert_eq!(placed, Ok(()));
    assert_eq!(name_at(&mut h, NEAR), "minecraft:glowstone");
    assert_eq!(name_at(&mut h, FAR), "", "column (5, 0) is out of a radius-2 view");
    h.remote.run(|ctx| ctx.teleport(sapling_api::PlayerId(1), [88.5, 64.0, 8.5]));
    let arrived = (0..40).any(|_| {
        h.tick_all();
        name_at(&mut h, FAR) == "minecraft:glowstone"
    });
    assert!(arrived, "placed once its column loaded");
}
