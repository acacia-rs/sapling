//! What crops, sugar cane and redstone need to stand on.

mod common;

use acacia_proto::packets::UpdateBlock;
use common::{Harness, all, click};
use sapling_api::{Block, Item, PlayerId, Slot};

const ALEX: PlayerId = PlayerId(1);

/// Alex in survival with a stack of `item`, Steve watching, and `blocks` set around them.
fn with(item: &str, blocks: &'static [([i32; 3], &'static str)]) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    let item = Item::new(item, 8);
    h.remote.run(move |ctx| {
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&item)));
        blocks.iter().for_each(|(pos, block)| assert!(ctx.set_block(*pos, &Block::new(*block)), "{block}"));
    });
    h.tick_all();
    h
}

/// The name of the block Steve sees placed after Alex clicks the top of `pos`.
fn placed_on(h: &mut Harness, pos: [i32; 3]) -> Option<&'static str> {
    h.receive(1, &click(pos, 1, None));
    let registry = sapling_world::BlockRegistry::vanilla();
    let updates = all::<UpdateBlock>(h.tick_all().to(2));
    updates.iter().filter(|update| update.layer == 0).find_map(|update| registry.get(registry.runtime_id_from_hash(update.block_runtime_id)?)).map(|state| state.name)
}

#[test]
fn seeds_go_into_farmland_only() {
    let mut h = with("minecraft:wheat_seeds", &[([2, 64, 0], "farmland")]);
    assert_eq!(placed_on(&mut h, [1, 64, 0]), None, "not on stone");
    assert_eq!(placed_on(&mut h, [2, 64, 0]), Some("minecraft:wheat"));
}

#[test]
fn sugar_cane_needs_water_beside_its_ground() {
    // Sand on the platform, beside water walled in so that it does not run.
    let mut h = with("minecraft:sugar_cane", &[
        ([2, 65, 0], "sand"),
        ([3, 65, 0], "water"),
        ([4, 65, 0], "stone"),
        ([3, 65, 1], "stone"),
        ([3, 65, -1], "stone"),
        ([2, 65, -2], "sand"),
    ]);
    assert_eq!(placed_on(&mut h, [2, 65, -2]), None, "no water beside");
    assert_eq!(placed_on(&mut h, [2, 65, 0]), Some("minecraft:reeds"));
    assert_eq!(placed_on(&mut h, [2, 66, 0]), Some("minecraft:reeds"), "on cane");

    // Stone where the water was: both canes break.
    h.remote.run(|ctx| ctx.set_game_mode(ALEX, sapling_api::GameMode::Creative));
    h.tick_all();
    h.receive(1, &click([3, 64, 0], 1, Some("minecraft:stone")));
    let registry = sapling_world::BlockRegistry::vanilla();
    let air = registry.network_hash(registry.air_id());
    let gone: Vec<_> = all::<UpdateBlock>(h.tick_all().to(2)).iter().filter(|update| update.block_runtime_id == air).map(|update| [update.position.x, update.position.y, update.position.z]).collect();
    assert_eq!(gone, [[2, 66, 0], [2, 67, 0]]);
}

#[test]
fn redstone_lies_on_a_top_and_not_on_a_low_slab() {
    let mut h = with("minecraft:redstone", &[([2, 65, 0], "oak_slab")]);
    assert_eq!(placed_on(&mut h, [2, 65, 0]), None);
    assert_eq!(placed_on(&mut h, [1, 64, 0]), Some("minecraft:redstone_wire"));
}
