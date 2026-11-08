//! A player under invisibility, as BDS showed it (research/captures/bds-bg2player.txt, bds-bf1give.txt):
//! its flags gain `INVISIBLE` in the tick-end SetEntityData that tells its effects (told.rs `show`),
//! and the tick after it and its viewers hear `disappeared` 0.9 over its feet. Going, the flags lose
//! `INVISIBLE` and gain `CAN_SHOW_NAMETAG` for good, and `reappeared` is heard. A raise tells nothing.
// TODO: a player coming back invisible (told.rs `greet`) is not shown so; BDS's join was not drilled.

use acacia_proto::packets::LevelSoundEvent;
use acacia_proto::types::{MetadataDictionaryItem, Vec3f};
use bevy_ecs::prelude::*;
use sapling_api::Effect;

use crate::components::{NetId, Position, Tick};
use crate::movement::Motion;

/// How far over a player's feet its vanishing is heard.
const HEARD_ABOVE: f32 = 0.9;

/// Players whose invisibility came (true) or went in a tick, to be heard the next.
#[derive(Resource, Default)]
struct Vanishing(Vec<(u64, Entity, bool)>);

pub(super) fn is_invisible(world: &World, player: Entity) -> bool {
    super::level(world, player, Effect::Invisibility).is_some()
}

/// The player's invisibility came (`on`) or went: its pose shows it, and it is heard next tick;
/// the flags item to tell.
pub(super) fn flip(world: &mut World, player: Entity, on: bool) -> Option<MetadataDictionaryItem> {
    let pose = world.get_mut::<Motion>(player)?.show_unseen(on);
    let synced = crate::entities::sync_tick(world);
    world.get_resource_or_insert_with(Vanishing::default).0.push((synced, player, on));
    crate::movement::flags_item(pose)
}

/// Plays the sounds of the players whose invisibility changed before this tick, at the sync.
pub(super) fn heard(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let Some(mut vanishing) = world.get_resource_mut::<Vanishing>() else { return };
    let (due, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut vanishing.0).into_iter().partition(|(when, ..)| *when < now);
    vanishing.0 = waiting;
    for (_, player, on) in due {
        let (Some(id), Some([x, y, z])) = (world.get::<NetId>(player).map(|id| id.0), world.get::<Position>(player).map(|position| position.feet)) else { continue };
        let sound_id = if on { "disappeared" } else { "reappeared" };
        let heard = LevelSoundEvent { sound_id: sound_id.into(), position: Vec3f { x, y: y + HEARD_ABOVE, z }, extra_data: -1, entity_type: "minecraft:player".into(), is_baby_mob: false, is_global: false, entity_unique_id: id as i64, fire_at_position: None };
        super::told::tell_at_sync(world, player, sapling_net::encode(&heard));
    }
}
