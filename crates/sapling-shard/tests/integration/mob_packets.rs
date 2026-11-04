//! The fifth wave's mob packets and saves: the body slot kept by a world folder
//! (research/captures/bds-bodysave.txt), a hidden mob's moves untold (bds-tickcircle6.txt,
//! bds-farmobs.txt), a new entity's first data and a death's armour (bds-bodyslot.txt), and a
//! slowed cow's head (bds-gaze.txt).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, MobArmorEquipment, MobEquipment, MoveEntityDelta, RequestChunkRadius, SetEntityData, UpdateAttributes};
use acacia_proto::types::MetadataDictionaryItemKey as Key;
use common::asking::ask;
use common::stored::{enter_stored, floor_with};
use common::{Harness, all, config, platform};
use sapling_api::{Effect, EntityId, Item, MobSlot, ShardCtx};
use sapling_shard::ShardConfig;
use sapling_world::{ChunkPos, World, WorldStorage, stored_actors};

fn put(h: &mut Harness, id: EntityId, slot: MobSlot, item: &'static str) {
    assert_eq!(ask(h, move |ctx| ctx.set_mob_item(id, slot, Some(&Item::new(item, 1)))), Ok(()), "{item}");
}

/// What the next two ticks sent Alex after `call` ran.
fn after(h: &mut Harness, call: impl FnOnce(&mut ShardCtx<'_>) + Send + 'static) -> Vec<RawPacket> {
    h.remote.run(call);
    let mut sent = h.tick_all().to(1).to_vec();
    sent.extend(h.tick_all().to(1).to_vec());
    sent
}

fn named(item: &acacia_proto::types::ItemV4) -> Option<&'static str> {
    (item.network_id != 0).then(|| sapling_data::item_name(item.network_id.into())).flatten()
}

/// The first data told of `id`: the `SetEntityData` holding more than its flag word.
fn first_data(sent: &[RawPacket], id: u64) -> Option<SetEntityData> {
    all::<SetEntityData>(sent).into_iter().find(|data| data.runtime_entity_id == id && data.metadata.len() > 2)
}

#[test]
fn the_body_slot_is_kept_by_the_world_folder_and_worn_again() {
    let storage = floor_with(Vec::new());
    let mut h = Harness::with(config(), World::with_storage(storage.clone()));
    enter_stored(&mut h, 1);
    let summon = |h: &mut Harness, kind: &'static str, event, x| ask(h, move |ctx| ctx.summon_as(None, kind, [x, 64.0, 8.5], 0.0, event)).expect(kind);
    let pig = summon(&mut h, "minecraft:pig", None, 4.5);
    let horse = summon(&mut h, "minecraft:horse", Some("minecraft:spawn_tame_adult"), 8.5);
    put(&mut h, pig, MobSlot::Body, "minecraft:iron_horse_armor");
    put(&mut h, horse, MobSlot::MountArmor, "minecraft:golden_horse_armor");
    put(&mut h, horse, MobSlot::Body, "minecraft:diamond_horse_armor");
    h.shard.save_worlds();
    let column = storage.load_column(ChunkPos::new(0, 0), 0).unwrap().expect("the column");
    let saved = stored_actors(&column.actors).unwrap();
    let kept = |kind: &str| saved.iter().find_map(|(actor, _, compound)| actor.as_ref().filter(|actor| actor.identifier == kind).map(|_| compound.clone())).expect(kind);
    let name = |stack: Option<sapling_world::ItemStack>| stack.map(|stack| stack.name);
    // bds-bodysave.txt: the body in Armor's fifth entry, slot 1's armour in ChestItems slot 1.
    assert_eq!(name(sapling_world::actor_body(&kept("minecraft:pig"))), Some("minecraft:iron_horse_armor".into()));
    assert_eq!(name(sapling_world::actor_body(&kept("minecraft:horse"))), Some("minecraft:diamond_horse_armor".into()));
    let slot_1 = sapling_world::actor_chest(&kept("minecraft:horse")).into_iter().find(|(slot, _)| *slot == 1).map(|(_, stack)| stack.name);
    assert_eq!(slot_1.as_deref(), Some("minecraft:golden_horse_armor"));

    let mut again = Harness::with(config(), World::with_storage(storage.clone()));
    let shown = enter_stored(&mut again, 1);
    let pig = all::<AddEntity>(&shown).into_iter().find(|added| added.entity_type == "minecraft:pig").expect("the pig").runtime_id;
    assert!(first_data(&shown, pig).is_none(), "a loaded mob has no first data");
    let sent = after(&mut again, move |ctx| assert!(ctx.kill_entity(EntityId(pig))));
    let worn: Vec<_> = all::<MobArmorEquipment>(&sent).into_iter().filter(|shown| shown.runtime_entity_id == pig).map(|shown| named(&shown.body)).collect();
    assert_eq!(worn, [Some("minecraft:iron_horse_armor")], "worn again, told at its death");
}

#[test]
fn a_new_mob_s_first_data_is_its_announcement_less_what_every_actor_is_made_with() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let sent = after(&mut h, |ctx| assert!(ctx.spawn_entity("minecraft:cow", [0.5, 65.0, 2.5]).is_some()));
    let added = all::<AddEntity>(&sent).into_iter().find(|added| added.entity_type == "minecraft:cow").expect("announced");
    let data = first_data(&sent, added.runtime_id).expect("its first data");
    let keys = |items: &[acacia_proto::types::MetadataDictionaryItem]| items.iter().map(|item| item.key).collect::<Vec<_>>();
    let kept: Vec<_> = keys(&added.metadata).into_iter().filter(|key| keys(&data.metadata).contains(key)).collect();
    assert_eq!(keys(&data.metadata), kept, "the announcement's items in its order");
    assert!(!kept.contains(&Key::Scale) && kept.contains(&Key::Flags) && kept.len() + 2 <= added.metadata.len(), "{kept:?}");
    assert_eq!(data.properties.ints.len(), added.properties.ints.len());
}

#[test]
fn a_dying_mob_s_viewers_see_its_hands_emptied_then_what_it_wears() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let husk = ask(&mut h, |ctx| {
        ctx.set_difficulty(sapling_api::Difficulty::Easy);
        ctx.spawn_entity("minecraft:husk", [0.5, 65.0, 4.5])
    })
    .expect("a husk");
    assert!(ask(&mut h, move |ctx| ctx.set_entity_armour(husk, 0, Some(&Item::new("minecraft:diamond_helmet", 1)))));
    put(&mut h, husk, MobSlot::Body, "minecraft:iron_horse_armor");
    let sent = after(&mut h, move |ctx| assert!(ctx.kill_entity(husk)));
    let hands: Vec<_> = all::<MobEquipment>(&sent).into_iter().filter(|held| held.runtime_entity_id == husk.0).map(|held| (held.slot, named(&held.item))).collect();
    assert_eq!(hands, [(0, None), (1, None)]);
    let worn = all::<MobArmorEquipment>(&sent).into_iter().find(|shown| shown.runtime_entity_id == husk.0).expect("its armour told");
    assert_eq!((named(&worn.helmet), named(&worn.body)), (Some("minecraft:diamond_helmet"), Some("minecraft:iron_horse_armor")));
}

#[test]
fn a_mob_hidden_from_a_player_is_not_told_to_it_but_its_teleport_is() {
    let mut h = Harness::with(ShardConfig { tick_distance: 8, max_view_radius: 8, ..config() }, platform());
    h.enter(1, "Alex");
    h.receive(1, &RequestChunkRadius { chunk_radius: 8, max_radius: 8 });
    (0..20).for_each(|_| drop(h.tick(1)));
    // Six chunks off: in Alex's view and ticked, not shown (bds-tickcircle6.txt); it falls and is hurt.
    let cow = ask(&mut h, |ctx| ctx.spawn_entity("minecraft:cow", [6.0 * 16.0 + 8.5, 65.0, 8.5])).expect("a cow");
    (0..3).for_each(|_| drop(h.tick(1)));
    h.remote.run(move |ctx| assert!(ctx.move_entity(cow, [6.0 * 16.0 + 8.5, 72.0, 12.5])));
    let sent: Vec<_> = (0..40).flat_map(|_| h.tick(1)).collect();
    let moves: Vec<_> = all::<MoveEntityDelta>(&sent).into_iter().filter(|delta| delta.runtime_entity_id == cow.0).collect();
    // bds-farmobs.txt: BDS tells a teleport to every player in view, the mob shown or not.
    assert_eq!(moves.iter().map(|delta| (delta.force_move, delta.z)).collect::<Vec<_>>(), [(true, Some(12.5))]);
    assert!(all::<UpdateAttributes>(&sent).iter().all(|told| told.runtime_entity_id != cow.0));
    assert!(ask(&mut h, move |ctx| ctx.entity(cow)).expect("still there").feet[1] < 71.0, "it was ticked");
}

#[test]
fn a_slowed_cow_turns_its_head_and_pitches_at_a_player_near_it() {
    let mut h = Harness::with(ShardConfig { mobs_stroll: true, ..config() }, platform());
    h.enter(1, "Alex");
    let cow = ask(&mut h, |ctx| {
        let cow = ctx.spawn_entity("minecraft:cow", [3.5, 65.0, 0.5])?;
        ctx.give_entity_effect(cow, Effect::Slowness, 2000, 255).then_some(cow)
    })
    .expect("a slowed cow");
    let sent: Vec<_> = (0..600).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let turns: Vec<_> = all::<MoveEntityDelta>(&sent).into_iter().filter(|delta| delta.runtime_entity_id == cow.0).collect();
    // bds-gaze.txt: up at the eyes (251 or 252), the head round to the player 7 a tick, the pitch back to 0.
    assert!(turns.iter().any(|delta| delta.rot_x.is_some_and(|pitch| (245..=254).contains(&pitch))), "{turns:?}");
    assert!(turns.iter().any(|delta| delta.rot_x == Some(0)), "the look ends");
    let heads: Vec<u8> = turns.iter().filter_map(|delta| delta.rot_z).collect();
    assert!(heads.iter().any(|head| (60..=68).contains(head)), "towards the player at -x: {heads:?}");
}

#[test]
fn a_slowed_cow_left_alone_glances_at_once_and_strolls_round_30_degrees_a_tick() {
    let mut h = Harness::with(ShardConfig { mobs_stroll: true, ..config() }, platform());
    h.enter(1, "Alex");
    let cow = ask(&mut h, |ctx| {
        let cow = ctx.spawn_entity("minecraft:cow", [9.5, 65.0, 0.5])?;
        ctx.give_entity_effect(cow, Effect::Slowness, 2000, 255).then_some(cow)
    })
    .expect("a slowed cow");
    let sent: Vec<_> = (0..1200).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let turns: Vec<_> = all::<MoveEntityDelta>(&sent).into_iter().filter(|delta| delta.runtime_entity_id == cow.0).collect();
    let step = |a: u8, b: u8| a.wrapping_sub(b).min(b.wrapping_sub(a));
    let steps = |values: Vec<u8>| values.windows(2).map(|pair| step(pair[0], pair[1])).collect::<Vec<_>>();
    // bds-glance.txt: a glance puts the head on its side in a tick (up to 53 of 256), a stroll turns 21 a tick.
    let heads = steps(turns.iter().filter_map(|delta| delta.rot_z).collect());
    assert!(heads.iter().any(|step| *step > 14), "{heads:?}");
    assert!(steps(turns.iter().filter_map(|delta| delta.rot_y).collect()).iter().all(|step| *step <= 22), "{turns:?}");
    // BDS's three idle cows sent 66, 72 and 61 moves in 60 s.
    assert!((20..=140).contains(&turns.len()), "{} moves", turns.len());
}
