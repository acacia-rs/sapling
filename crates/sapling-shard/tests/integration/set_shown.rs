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

/// research/captures/bds-s7ents.txt: what each set block's entity showed at once and a tick later.
#[test]
fn a_set_block_shows_its_entity_as_bds_s_did_and_some_again() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.tick_all();
    let version = || "BlockEntityVersion".to_owned();
    let cases: [(&str, Option<Vec<&str>>, usize); 12] = [
        ("minecraft:frame", Some(vec!["id"]), 1),
        ("minecraft:jukebox", Some(vec!["FinishedRecording", "TicksPlaying", "id"]), 1),
        ("minecraft:lodestone", Some(vec!["id"]), 0),
        ("minecraft:trial_spawner", Some(Vec::new()), 0),
        ("minecraft:crafter", Some(vec!["crafting_ticks_remaining", "disabled_slots"]), 0),
        ("minecraft:standing_sign", Some(vec!["BackText", "FrontText", "IsWaxed", "LockedForEditingBy", "id"]), 1),
        ("minecraft:cauldron", Some(vec!["Items", "PotionId", "PotionType", "id"]), 1),
        ("minecraft:undyed_shulker_box", Some(vec!["Findable", "Items", "facing", "id"]), 0),
        ("minecraft:enchanting_table", Some(vec!["id", "rott"]), 0),
        ("minecraft:furnace", None, 0),
        ("minecraft:hopper", None, 0),
        ("minecraft:noteblock", None, 0),
    ];
    for (index, (block, shown, again)) in cases.into_iter().enumerate() {
        let at = [index as i32 - 6, 65, 3];
        h.remote.run(move |ctx| assert!(ctx.set_block(at, &Block::new(block))));
        let first = all::<BlockEntityData>(h.tick_all().to(1));
        let unversioned = |entity: &BlockEntityData| keys(entity).into_iter().filter(|key| *key != version() && !["x", "y", "z"].contains(&key.as_str())).collect::<Vec<_>>();
        let expected: Vec<Vec<String>> = shown.iter().map(|keys| keys.iter().map(|key| key.to_string()).collect()).collect();
        assert_eq!(first.iter().map(unversioned).collect::<Vec<_>>(), expected, "{block}");
        let later = all::<BlockEntityData>(h.tick_all().to(1));
        assert_eq!(later.len(), again, "{block}");
    }
}
