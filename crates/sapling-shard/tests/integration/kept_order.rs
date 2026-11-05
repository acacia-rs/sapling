//! The order a world folder's mobs are shown in (mobs.rs `FromFolder`).

use crate::common;

use acacia_proto::nbt::Value;
use acacia_proto::packets::AddEntity;
use common::stored::{actor, enter_stored, floor_with};
use common::{Harness, all, config};
use sapling_world::World;

/// bds-s3far.txt and bds-ad3reload.txt: the cows of a column come back newest first.
#[test]
fn a_world_folders_cows_are_shown_newest_first() {
    let cows: Vec<u8> = (0..4).flat_map(|at| actor(-30 + at, "minecraft:cow", [3.5 + 2.0 * at as f32, 64.0, 3.5], ("Age", Value::Int(0)))).collect();
    let mut h = Harness::with(config(), World::with_storage(floor_with(cows)));
    let sent = enter_stored(&mut h, 1);
    let shown: Vec<u64> = all::<AddEntity>(&sent).into_iter().filter(|added| added.entity_type == "minecraft:cow").map(|added| added.runtime_id).collect();
    assert_eq!(shown.len(), 4, "{shown:?}");
    assert!(shown.windows(2).all(|pair| pair[0] > pair[1]), "newest first: {shown:?}");
}
