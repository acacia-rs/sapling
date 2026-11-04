//! What a block with an entity set by a handler (`/setblock`) sends, as BDS's
//! (research/captures/bds-setshown.txt, bds-potsapling.txt; harness: common/mod.rs).

use crate::common;

use acacia_proto::nbt::Value;
use acacia_proto::packets::BlockEntityData;
use common::{Harness, all};
use sapling_api::Block;

fn keys(entity: &BlockEntityData) -> Vec<String> {
    let Value::Compound(fields) = entity.nbt.decode().unwrap().value else { panic!("not a compound") };
    fields.into_iter().map(|(key, _)| key.to_string()).collect()
}

#[test]
fn a_set_pot_or_bell_shows_its_entity_versioned_and_sorted() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.tick_all();
    h.remote.run(|ctx| assert!(ctx.set_block([1, 65, 2], &Block::new("minecraft:flower_pot"))));
    let [pot]: [BlockEntityData; 1] = all::<BlockEntityData>(h.tick_all().to(1)).try_into().expect("the pot's entity alone");
    assert_eq!(keys(&pot), ["BlockEntityVersion", "id", "x", "y", "z"]);
    h.remote.run(|ctx| assert!(ctx.set_block([1, 65, 4], &Block::new("minecraft:bell"))));
    let [bell]: [BlockEntityData; 1] = all::<BlockEntityData>(h.tick_all().to(1)).try_into().expect("the bell's entity alone");
    assert_eq!(keys(&bell), ["BlockEntityVersion", "Direction", "Ringing", "Ticks", "id", "x", "y", "z"]);
}
