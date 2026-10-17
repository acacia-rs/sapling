//! Entities through the plugin API: spawned, falling, listed, pushed, moved and removed
//! (harness: common/mod.rs).

mod common;

use std::sync::{Arc, Mutex};

use acacia_proto::nbt::Value;
use acacia_proto::Packet;
use acacia_proto::packets::{
    AddEntity, AddItemEntity, EntityEvent, EntityEventEventId, LevelEvent, LevelEventEvent, LevelSoundEvent, MoveEntityDelta, RemoveEntity, SetEntityData, UpdateAttributes, UpdateBlock,
};
use acacia_proto::types::MetadataDictionaryItemValue as MetaValue;
use common::stored::{actor, enter_stored, floor_with};
use common::{Harness, all, hash};
use sapling_api::{Block, EntityId, EntityInfo};
use sapling_world::{ChunkPos, World, WorldStorage};

/// Runs `ask` on the shard and returns what it found.
fn ask<T: Send + 'static>(h: &mut Harness, ask: impl FnOnce(&mut sapling_api::ShardCtx<'_>) -> T + Send + 'static) -> T {
    let found = Arc::new(Mutex::new(None));
    let out = found.clone();
    h.remote.run(move |ctx| *out.lock().unwrap() = Some(ask(ctx)));
    h.tick_all();
    found.lock().unwrap().take().expect("the handler ran")
}

fn info(h: &mut Harness, id: EntityId) -> Option<EntityInfo> {
    ask(h, move |ctx| ctx.entity(id))
}

#[test]
fn a_spawned_mob_falls_onto_the_floor_and_is_listed() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let pig = ask(&mut h, |ctx| ctx.spawn_entity("minecraft:pig", [3.5, 70.0, 0.5])).expect("a pig");
    assert_eq!(ask(&mut h, |ctx| [ctx.spawn_entity("minecraft:item", [0.5, 66.0, 0.5]), ctx.spawn_entity("sapling:nothing", [0.5, 66.0, 0.5])]), [None, None], "the game's own kinds, and none at all");

    // The spawn's own tick went to the asking; the pig is shown to a later joiner as a pig.
    let sent: Vec<_> = (0..40).flat_map(|_| h.tick(1)).collect();
    let pig_id = pig.0;
    let seen = h.enter(2, "Steve");
    assert!(all::<AddEntity>(seen.to(2)).iter().any(|entity| entity.entity_type == "minecraft:pig" && entity.runtime_id == pig_id && entity.position.y == 65.0));
    let heights: Vec<f32> = all::<MoveEntityDelta>(&sent).iter().filter(|moved| moved.runtime_entity_id == pig_id).filter_map(|moved| moved.y).collect();
    assert!(heights.last() == Some(&65.0), "down onto the stone: {heights:?}");
    let now = info(&mut h, pig).expect("still there");
    assert_eq!((now.kind.as_str(), now.feet, now.on_ground), ("minecraft:pig", [3.5, 65.0, 0.5], true));
    assert_eq!(ask(&mut h, |ctx| ctx.entities()).iter().map(|entity| entity.id).collect::<Vec<_>>(), [pig]);
}

#[test]
fn a_spawned_mob_is_heard_at_its_head_after_it_is_shown_as_on_bds() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.remote.run(|ctx| assert!(ctx.spawn_entity("minecraft:pig", [3.5, 65.0, 0.5]).is_some()));
    let sent = h.tick_all();
    let ids: Vec<u32> = sent.to(2).iter().map(|packet| packet.id).filter(|id| [AddEntity::ID, LevelSoundEvent::ID].contains(id)).collect();
    assert_eq!(ids, [AddEntity::ID, LevelSoundEvent::ID]);
    let sound = &all::<LevelSoundEvent>(sent.to(2))[0];
    // A pig is 0.9 wide and high; its head 0.85 of that up and, an animal's, forward.
    let at = [sound.position.x, sound.position.y, sound.position.z];
    assert!((sound.sound_id.as_str(), sound.entity_type.as_str()) == ("spawn", "minecraft:pig") && (0..3).all(|axis| (at[axis] - [3.5, 65.765, 1.265][axis]).abs() < 1e-4), "{sound:?}");
}

#[test]
fn an_item_in_lava_fizzes_burns_and_goes_as_on_bds() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    // Dirt broken over lava set in the floor, walled in so that its item falls into the lava.
    h.remote.run(|ctx| {
        for y in 63..=66 {
            for [dx, dz] in [[-1, 0], [1, 0], [0, -1], [0, 1]] {
                assert!(ctx.set_block([6 + dx, y, dz], &Block::new("stone")));
            }
        }
        assert!(ctx.set_block([6, 63, 0], &Block::new("stone")) && ctx.set_block([6, 64, 0], &Block::new("lava")) && ctx.set_block([6, 65, 0], &Block::new("dirt")) && ctx.break_block([6, 65, 0]));
    });
    let ticks: Vec<_> = (0..40).map(|_| h.tick_all().to(2).to_vec()).collect();
    let sent: Vec<_> = ticks.concat();
    let item = all::<AddItemEntity>(&sent)[0].runtime_entity_id;
    let when = |found: &dyn Fn(&[acacia_proto::RawPacket]) -> bool| ticks.iter().position(|sent| found(sent));
    let burning = when(&|sent| all::<SetEntityData>(sent).iter().any(|data| data.runtime_entity_id == item && matches!(data.metadata[0].value, MetaValue::Flags(flags) if flags.0 & 1 == 1)));
    let gone = when(&|sent| !all::<RemoveEntity>(sent).is_empty());
    let fizzes: Vec<usize> = ticks.iter().enumerate().filter(|(_, sent)| all::<LevelSoundEvent>(sent).iter().any(|sound| sound.sound_id == "fizz")).map(|(tick, _)| tick).collect();
    let burning = burning.expect("set on fire");
    // On fire as it is lifted, a fizz a tick late for each of its two ticks in lava, gone with the second.
    assert_eq!((fizzes, gone), (vec![burning + 1, burning + 2], Some(burning + 2)));
}

#[test]
fn orbs_at_a_player_are_taken_two_ticks_apart_and_go_the_tick_after_as_on_bds() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let feet = ask(&mut h, |ctx| ctx.position(sapling_api::PlayerId(1)).expect("Alex").feet);
    h.remote.run(move |ctx| assert!((0..2).all(|_| ctx.spawn_entity("minecraft:xp_orb", feet).is_some()), "an orb is an entity of its own"));
    // From the tick they appear in: BDS took one then.
    let ticks: Vec<_> = (0..8).map(|_| h.tick_all().to(1).to_vec()).collect();
    let when = |found: &dyn Fn(&[acacia_proto::RawPacket]) -> usize| -> Vec<usize> { ticks.iter().enumerate().flat_map(|(tick, sent)| std::iter::repeat_n(tick, found(sent))).collect() };
    let taken = when(&|sent| all::<LevelEvent>(sent).iter().filter(|event| event.event == LevelEventEvent::SoundOrb).count());
    let gone = when(&|sent| all::<RemoveEntity>(sent).len());
    assert_eq!(taken.len(), 2, "{taken:?}");
    assert_eq!((taken[1] - taken[0], gone), (2, vec![taken[0] + 1, taken[1] + 1]));
}

#[test]
fn a_mob_spawned_on_a_plate_presses_it_the_tick_after_as_on_bds() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    let plate = [3, 65, 0];
    h.remote.run(move |ctx| assert!(ctx.set_block(plate, &Block::new("wooden_pressure_plate"))));
    h.tick_all();
    h.remote.run(move |ctx| assert!(ctx.spawn_entity("minecraft:pig", [3.5, 65.0, 0.5]).is_some()));
    let at_plate = |sent: &[acacia_proto::RawPacket]| all::<UpdateBlock>(sent).iter().filter(|u| [u.position.x, u.position.y, u.position.z] == plate).count();
    let (spawned, after) = (h.tick_all().to(2).to_vec(), h.tick_all().to(2).to_vec());
    assert_eq!((all::<AddEntity>(&spawned).len(), at_plate(&spawned), at_plate(&after)), (1, 0, 1));
}

#[test]
fn a_killed_mob_dies_leaves_its_loot_and_goes_a_second_later_as_on_bds() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    let pig = ask(&mut h, |ctx| ctx.spawn_entity("minecraft:pig", [3.5, 65.0, 0.5])).expect("a pig");
    (0..5).for_each(|_| drop(h.tick_all()));
    h.remote.run(move |ctx| assert!(ctx.kill_entity(pig) && !ctx.kill_entity(pig), "dies once"));
    let died = h.tick_all();
    let events: Vec<_> = all::<EntityEvent>(died.to(2)).iter().map(|event| (event.runtime_entity_id, event.event_id.clone())).collect();
    assert_eq!(events, [(pig.0, EntityEventEventId::DeathAnimation)]);
    let loot = all::<AddItemEntity>(died.to(2));
    assert!(!loot.is_empty() && loot.iter().all(|item| (item.position.y - 65.075).abs() < 1e-4 && item.velocity.y == 0.2), "porkchops at the pig: {loot:?}");
    // 0.05 into the floor, each is lifted onto it and stays, as BDS's did.
    let porkchop = loot[0].runtime_entity_id;
    let ticks: Vec<_> = (1..=25).map(|_| h.tick_all().to(2).to_vec()).collect();
    let moves: Vec<_> = std::iter::once(died.to(2)).chain(ticks.iter().map(Vec::as_slice)).flat_map(all::<MoveEntityDelta>).filter(|moved| moved.runtime_entity_id == porkchop).map(|moved| (moved.x, moved.y, moved.on_ground)).collect();
    assert_eq!(moves, [(None, Some(65.125), true)]);
    let gone: Vec<usize> = (1..=25).filter(|tick| all::<RemoveEntity>(&ticks[tick - 1]).iter().any(|remove| remove.entity_id_self == pig.0 as i64)).collect();
    assert_eq!(gone, [21], "gone 21 ticks after it died");

    let calf = ask(&mut h, |ctx| {
        assert!(ctx.set_game_rule("doMobLoot", sapling_api::RuleValue::Bool(false)));
        ctx.spawn_entity("minecraft:pig", [5.5, 65.0, 0.5])
    })
    .expect("another pig");
    h.remote.run(move |ctx| assert!(ctx.kill_entity(calf)));
    assert!(all::<AddItemEntity>(h.tick_all().to(2)).is_empty(), "no loot without doMobLoot");
}

#[test]
fn a_world_folders_mobs_move_and_are_saved_where_they_are() {
    let actors = [
        actor(-2, "minecraft:cow", [3.5, 70.0, 3.5], ("Variant", Value::Int(1))),
        actor(-3, "minecraft:horse", [6.5, 64.0, 6.5], ("Variant", Value::Int(2))),
        actor(-4, "minecraft:painting", [7.0, 66.0, 0.03], ("Motif", Value::String("Kebab".into()))),
    ]
    .concat();
    let storage = floor_with(actors);
    let mut h = Harness::with(common::config(), World::with_storage(storage.clone()));
    enter_stored(&mut h, 1);
    // The cow falls onto the floor; the horse is taken away; a pig is spawned.
    let horse = ask(&mut h, |ctx| ctx.entities().into_iter().find(|entity| entity.kind == "minecraft:horse").map(|horse| horse.id)).expect("the horse is an entity");
    assert!(ask(&mut h, move |ctx| ctx.remove_entity(horse) && ctx.spawn_entity("minecraft:pig", [1.5, 64.0, 1.5]).is_some()));
    (0..40).for_each(|_| drop(h.tick(1)));
    h.shard.save_worlds();

    let stored = storage.load_column(ChunkPos::new(0, 0), 0).unwrap().expect("the column");
    let mut kept: Vec<(String, [f32; 3])> = sapling_world::actors(&stored.actors).unwrap().into_iter().map(|actor| (actor.identifier, actor.position)).collect();
    kept.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(kept, [("minecraft:cow".to_string(), [3.5, 64.0, 3.5]), ("minecraft:painting".into(), [7.0, 66.0, 0.03]), ("minecraft:pig".into(), [1.5, 64.0, 1.5])]);
    let cow = sapling_world::actors(&stored.actors).unwrap().into_iter().find(|actor| actor.identifier == "minecraft:cow").unwrap();
    assert_eq!((cow.variant, cow.health), (1, Some(7.0)), "the rest of what was stored stays; its 6 block fall cost 3");
}

#[test]
fn a_world_folders_falling_blocks_fall_on_and_are_saved_until_placed() {
    let registry = sapling_world::BlockRegistry::vanilla();
    let (sand, sand_hash) = registry.states_of("minecraft:sand").next().map(|(id, state)| (id, state.network_hash)).unwrap();
    let falling = |id, at| sapling_world::new_falling_block_actor(id, sand_hash, &registry, at, [0.0; 3], 1).unwrap();
    let storage = floor_with([falling(-2, [9.5, 70.0, 9.5]), falling(-3, [12.5, 300.0, 12.5])].concat());
    let mut h = Harness::with(common::config(), World::with_storage(storage.clone()));
    enter_stored(&mut h, 1);
    let kept = |storage: &sapling_world::MemoryStorage| {
        let stored = storage.load_column(ChunkPos::new(0, 0), 0).unwrap().expect("the column");
        let falling: Vec<(i64, f32)> = sapling_world::actors(&stored.actors).unwrap().into_iter().map(|actor| (actor.unique_id, actor.position[1])).collect();
        (falling, stored.chunk.block(9, 64, 9) == sand)
    };

    (0..60).for_each(|_| drop(h.tick(1)));
    h.shard.save_worlds();
    let (falling, placed) = kept(&storage);
    assert!(placed, "the low one landed on the floor");
    assert!(matches!(falling[..], [(-3, y)] if y < 300.0 && y > 64.0), "only the high one is kept, lower than it was: {falling:?}");
}

#[test]
fn a_mob_is_pushed_moved_and_removed_and_presses_a_plate() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let plate = [5, 65, 0];
    let pig = ask(&mut h, move |ctx| {
        assert!(ctx.set_block(plate, &Block::new("stone_pressure_plate")));
        ctx.spawn_entity("minecraft:pig", [1.5, 65.0, 0.5])
    })
    .expect("a pig");

    // Pushed east along the floor: ground friction stops it after a few blocks.
    assert!(ask(&mut h, move |ctx| ctx.set_velocity(pig, [0.5, 0.0, 0.0])));
    (0..30).for_each(|_| drop(h.tick(1)));
    let slid = info(&mut h, pig).unwrap();
    assert!(slid.feet[0] > 2.0 && slid.velocity[0].abs() < 1e-3, "{slid:?}");

    // Put onto the plate: a mob presses stone.
    assert!(ask(&mut h, move |ctx| ctx.move_entity(pig, [5.5, 65.0, 0.5])));
    let sent: Vec<_> = (0..3).flat_map(|_| h.tick(1)).collect();
    let up = hash("minecraft:stone_pressure_plate");
    let pressed = all::<UpdateBlock>(&sent).iter().any(|u| [u.position.x, u.position.y, u.position.z] == plate && u.block_runtime_id != up);
    assert!(pressed, "the plate went down");

    h.remote.run(move |ctx| assert!(ctx.remove_entity(pig)));
    let gone = all::<RemoveEntity>(&h.tick(1));
    assert_eq!(gone.iter().map(|remove| remove.entity_id_self).collect::<Vec<_>>(), [pig.0 as i64]);
    assert_eq!((info(&mut h, pig), ask(&mut h, move |ctx| ctx.remove_entity(pig))), (None, false));
}

#[test]
fn a_pig_in_lava_burns_is_hurt_and_dies_cooked_as_on_bds() {
    const COOKED_PORKCHOP: i16 = 263;
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.remote.run(|ctx| {
        for y in 63..=66 {
            for [dx, dz] in [[-1, 0], [1, 0], [0, -1], [0, 1]] {
                assert!(ctx.set_block([6 + dx, y, dz], &Block::new("stone")));
            }
        }
        assert!(ctx.set_block([6, 63, 0], &Block::new("stone")) && ctx.set_block([6, 64, 0], &Block::new("lava")));
        assert!(ctx.spawn_entity("minecraft:pig", [6.5, 65.0, 0.5]).is_some());
    });
    let ticks: Vec<_> = (0..30).map(|_| h.tick_all().to(2).to_vec()).collect();
    // Counted from the tick the pig is shown in.
    let shown = ticks.iter().position(|sent| !all::<AddEntity>(sent).is_empty()).expect("a pig");
    let ticks = &ticks[shown..];
    let events: Vec<(usize, EntityEventEventId, i32)> = ticks.iter().enumerate().flat_map(|(tick, sent)| all::<EntityEvent>(sent).into_iter().map(move |event| (tick, event.event_id, event.data))).collect();
    // research/captures/bds-burn.txt: fire's 1 (cause 6), then lava's 4 (cause 8) by what it adds, every 10
    // ticks. A tick later than there: a plugin's spawn comes after the tick's move, a command's before.
    let hurt = EntityEventEventId::HurtAnimation;
    assert_eq!(events, [(2, hurt.clone(), 6), (4, hurt.clone(), 8), (14, hurt.clone(), 6), (14, hurt.clone(), 8), (24, hurt, 6), (24, EntityEventEventId::DeathAnimation, 0)]);
    let healths: Vec<(usize, f32)> = ticks.iter().enumerate().flat_map(|(tick, sent)| all::<UpdateAttributes>(sent).into_iter().map(move |update| (tick, update.attributes[0].current))).collect();
    assert_eq!(healths, [(2, 9.0), (5, 6.0), (14, 5.0), (15, 2.0), (24, 1.0), (25, 0.0)], "lava's harm is told a tick late");
    let lit = ticks.iter().position(|sent| all::<SetEntityData>(sent).iter().any(|data| matches!(data.metadata[0].value, MetaValue::Flags(flags) if flags.0 & 1 == 1)));
    assert_eq!(lit, Some(2));
    let loot: Vec<i16> = ticks.concat().iter().filter(|packet| packet.id == AddItemEntity::ID).map(|packet| packet.decode::<AddItemEntity>().unwrap().item.network_id).collect();
    assert_eq!(loot, [COOKED_PORKCHOP]);
}

#[test]
fn a_cow_dropped_five_blocks_is_hurt_by_two_and_heard_landing_as_on_bds() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.remote.run(|ctx| assert!(ctx.spawn_entity("minecraft:cow", [3.5, 70.0, 0.5]).is_some()));
    let ticks: Vec<_> = (0..20).map(|_| h.tick_all().to(2).to_vec()).collect();
    let landed = ticks.iter().position(|sent| !all::<EntityEvent>(sent).is_empty()).expect("hurt");
    let events: Vec<_> = all::<EntityEvent>(&ticks[landed]).into_iter().map(|event| (event.event_id, event.data, event.fire_at_position.map(|at| [at.x, at.y, at.z]))).collect();
    // research/captures/bds-harm.txt: cause 5 naming where it landed, then dust; the sounds a tick later.
    assert_eq!(events, [(EntityEventEventId::HurtAnimation, 5, Some([3.5, 65.0, 0.5])), (EntityEventEventId::DustParticles, 0, None)]);
    assert_eq!(all::<UpdateAttributes>(&ticks[landed]).iter().map(|update| update.attributes[0].current).collect::<Vec<_>>(), [8.0]);
    let sounds: Vec<_> = all::<LevelSoundEvent>(&ticks[landed + 1]).into_iter().map(|sound| (sound.sound_id, sound.extra_data)).collect();
    assert_eq!(sounds, [("fall.small".to_owned(), -1), ("fall".to_owned(), hash("minecraft:stone") as i32)]);
}

#[test]
fn water_puts_a_burning_cow_out_with_steam_and_a_fizz_as_on_bds() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.remote.run(|ctx| assert!(ctx.set_block([3, 65, 0], &Block::new("fire")) && ctx.spawn_entity("minecraft:cow", [3.5, 65.0, 0.5]).is_some()));
    (0..5).for_each(|_| drop(h.tick_all()));
    h.remote.run(|ctx| assert!(ctx.set_block([3, 65, 0], &Block::new("water"))));
    let ticks: Vec<_> = (0..5).map(|_| h.tick_all().to(2).to_vec()).collect();
    let doused = ticks.iter().position(|sent| !all::<LevelEvent>(sent).is_empty()).expect("steam");
    // research/captures/bds-douse.txt: a cow, 0.9 wide and 1.3 high.
    let steam: Vec<_> = all::<LevelEvent>(&ticks[doused]).into_iter().map(|event| (event.event, [event.position.x, event.position.y, event.position.z], event.data)).collect();
    assert_eq!(steam, [(LevelEventEvent::ParticleFizzEffect, [3.5 - 0.45, 65.0, 0.5 - 0.45], 513)]);
    let fizz: Vec<_> = all::<LevelSoundEvent>(&ticks[doused + 1]).into_iter().filter(|sound| sound.sound_id == "fizz").map(|sound| (sound.position.y, sound.extra_data)).collect();
    assert_eq!(fizz, [(65.65, -1)]);
}
