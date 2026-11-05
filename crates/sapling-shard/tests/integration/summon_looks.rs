//! `/summon`'s pitch and a summoned mob's name tag, as BDS showed them (research/captures/bds-h1summon.txt).

use std::sync::{Arc, Mutex};

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, MoveEntityDelta};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValueDefault as Plain};
use common::raising::plain_in;
use common::{Harness, all};
use sapling_api::{Difficulty, EntityId, GameMode, PlayerId, ShardCtx};

/// What `make` made in the next tick, and what the player was sent over 8 ticks from it.
fn made(make: impl FnOnce(&mut ShardCtx<'_>) -> Option<EntityId> + Send + 'static) -> (EntityId, Vec<RawPacket>) {
    // Mobs turn their heads only where they stroll.
    let mut h = Harness::with(sapling_shard::ShardConfig { mobs_stroll: true, ..common::config() }, common::platform());
    h.enter(1, "Alex");
    // A spectator is not looked at, so nothing but the summon pitches the mob.
    h.remote.run(|ctx| {
        ctx.set_game_mode(PlayerId(1), GameMode::Spectator);
        ctx.set_difficulty(Difficulty::Easy);
    });
    h.tick_all();
    let found = Arc::new(Mutex::new(None));
    let out = found.clone();
    h.remote.run(move |ctx| *out.lock().unwrap() = make(ctx));
    let sent = (0..8).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let made = found.lock().unwrap().take().expect("made");
    (made, sent)
}

#[test]
fn a_summoned_pig_is_announced_pitched_and_comes_level_10_degrees_a_tick() {
    let (pig, sent) = made(|ctx| ctx.summon_as(None, "minecraft:pig", [2.5, 65.0, 2.5], 90.0, 45.0, None));
    let added = all::<AddEntity>(&sent).into_iter().find(|added| added.runtime_id == pig.0).expect("announced");
    assert_eq!((added.pitch, added.yaw), (45.0, 90.0));
    let pitches: Vec<u8> = all::<MoveEntityDelta>(&sent).into_iter().filter(|delta| delta.runtime_entity_id == pig.0).filter_map(|delta| delta.rot_x).collect();
    assert_eq!(pitches, [24, 17, 10, 3, 0]);
}

#[test]
fn a_mob_named_as_it_is_summoned_is_announced_with_its_name() {
    let (zombie, sent) = made(|ctx| {
        let zombie = ctx.summon_as(None, "minecraft:zombie", [2.5, 65.0, 2.5], 0.0, 0.0, None)?;
        ctx.name_entity(zombie, "Rider").then_some(zombie)
    });
    let added = all::<AddEntity>(&sent).into_iter().find(|added| added.runtime_id == zombie.0).expect("announced");
    assert_eq!(plain_in(&added.metadata, Key::Nametag), Some(Plain::String("Rider".into())));
}
