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
    h.remote.run(|ctx| assert!(ctx.set_block([-7, 65, 3], &Block::new("minecraft:stone"))));
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

/// research/captures/bds-bkuses.txt: a set head plays `place` after its entity, at the cell's middle,
/// naming the block as BDS's did (zombie_head facing up: 173921225); a set stone plays nothing.
#[test]
fn a_set_head_plays_its_place_sound_and_a_set_stone_none() {
    use acacia_proto::packets::LevelSoundEvent;
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.tick_all();
    h.remote.run(|ctx| assert!(ctx.update_block([2, 65, 2], &Block::new("minecraft:zombie_head").with("facing_direction=1"))));
    let sent = h.tick_all().to(1).to_vec();
    let heard: Vec<(String, [f32; 3], i32)> = all::<LevelSoundEvent>(&sent).into_iter().map(|sound| (sound.sound_id, [sound.position.x, sound.position.y, sound.position.z], sound.extra_data)).collect();
    assert_eq!(heard, [("place".to_owned(), [2.5, 65.5, 2.5], 173_921_225)]);
    h.remote.run(|ctx| assert!(ctx.update_block([2, 65, 4], &Block::new("minecraft:stone"))));
    assert!(all::<LevelSoundEvent>(h.tick_all().to(1)).is_empty());
}

/// research/captures/bds-e3sets.txt: a frame set with nothing behind it showed its break at once
/// and was gone a tick later, its item dropped.
#[test]
fn a_set_frame_with_nothing_to_hang_on_breaks_a_tick_later() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.tick_all();
    h.remote.run(|ctx| assert!(ctx.set_block([2, 65, 3], &Block::new("minecraft:frame"))));
    let set = h.tick_all().to(1).to_vec();
    assert_eq!(all::<acacia_proto::packets::LevelEvent>(&set).len(), 1, "its break shown with it");
    let later = h.tick_all().to(1).to_vec();
    assert_eq!(common::block_changes(&later), [([2, 65, 3], common::hash("minecraft:air"))]);
    assert_eq!(all::<acacia_proto::packets::AddItemEntity>(&later).len(), 1, "its item");
}

/// research/captures/bds-t3frame.txt: a frame given an item, then turned, is heard at once and
/// shown, block and entity, a tick later.
#[test]
fn a_clicked_frame_is_heard_at_once_and_shown_a_tick_later() {
    const FRAME: [i32; 3] = [1, 65, 0];
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_game_mode(sapling_api::PlayerId(1), sapling_api::GameMode::Creative);
        assert!(ctx.set_item(sapling_api::PlayerId(1), sapling_api::Slot::Main(0), Some(&sapling_api::Item::new("minecraft:diamond", 1))));
        assert!(ctx.update_block([2, 65, 0], &Block::new("minecraft:stone")));
        assert!(ctx.update_block(FRAME, &Block::new("minecraft:frame").with("facing_direction=4,item_frame_map_bit=0,item_frame_photo_bit=0")));
    });
    (0..2).for_each(|_| drop(h.tick_all()));
    for _ in 0..2 {
        h.receive(1, &common::click(FRAME, 4, None));
        let heard = h.tick_all().to(1).to_vec();
        assert_eq!((all::<acacia_proto::packets::LevelEvent>(&heard).len(), all::<BlockEntityData>(&heard).len()), (1, 0));
        let shown = h.tick_all().to(1).to_vec();
        let block = common::block_changes(&shown).into_iter().filter(|(pos, _)| *pos == FRAME).count();
        assert_eq!((block, all::<BlockEntityData>(&shown).len()), (1, 1));
    }
}
