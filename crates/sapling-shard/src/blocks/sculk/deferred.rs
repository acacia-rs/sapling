//! Game events BDS posts a tick after the request that caused them: a `/tp`'s arrival and a
//! bucket's pour show their vibration a tick after the command's answer or the poured water,
//! where a hand-placed or broken block's and a drink's come in their own tick (research/captures/
//! bds-ai1vib.txt, bds-v2tp.txt, bds-uusculk.txt).

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::GameEvent;

/// An event put off: its world, where, what, whether a player made it, and who did where known.
#[derive(Resource, Default)]
pub(super) struct Deferred(Vec<(Entity, [f32; 3], GameEvent, bool, Option<Entity>)>);

fn defer(world: &mut World, layer: Entity, at: [f32; 3], event: GameEvent, by_player: bool, source: Option<Entity>) {
    world.get_resource_or_insert_with(Deferred::default).0.push((layer, at, event, by_player, source));
}

/// An entity `/tp` moved from `from` to `to` is heard at the cell it arrives in, the next tick; one
/// put where it stood is not (research/captures/bds-v2tp.txt).
pub(crate) fn teleported(world: &mut World, layer: Entity, from: [f32; 3], to: [f32; 3], by_player: bool) {
    if from != to {
        defer(world, layer, to, GameEvent::Teleport, by_player, None);
    }
}

/// `player`'s bucket poured into `pos`: heard from its middle the next tick.
pub(crate) fn poured(world: &mut World, layer: Entity, pos: BlockPos, player: Entity) {
    defer(world, layer, pos.map(|c| c as f32 + 0.5), GameEvent::FluidPlace, true, Some(player));
}

/// `event` from the middle of the block at `pos`, made by `source` (a player, a mob or nothing),
/// heard the next tick: a potent sulfur's eruption that a change beside it started or ended
/// (bds-ag2lava.txt), a hurt or dead body.
/// Listeners take it as made by no player, as before wardens were told its source.
pub(crate) fn vibrate_next_tick(world: &mut World, layer: Entity, pos: BlockPos, event: GameEvent, source: Option<Entity>) {
    defer(world, layer, pos.map(|c| c as f32 + 0.5), event, false, source);
}

/// The events put off last tick, posted now; one whose source is gone is no one's.
pub(super) fn post(world: &mut World) {
    let Some(mut deferred) = world.get_resource_mut::<Deferred>() else { return };
    for (layer, at, event, by_player, source) in std::mem::take(&mut deferred.0) {
        let source = source.filter(|source| world.get_entity(*source).is_ok());
        super::vibrate_from(world, layer, at, event, by_player, source);
    }
}
