//! Entities through the plugin API: spawned, falling, listed, pushed, moved and removed
//! (harness: common/mod.rs).

mod common;

use std::sync::{Arc, Mutex};

use acacia_proto::nbt::Value;
use acacia_proto::Packet;
use acacia_proto::packets::{AddEntity, LevelSoundEvent, MoveEntityDelta, RemoveEntity, UpdateBlock};
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
    assert_eq!(cow.variant, 1, "the rest of what was stored stays");
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
