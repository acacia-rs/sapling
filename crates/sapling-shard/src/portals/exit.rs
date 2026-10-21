//! Home through the End's exit portal, as BDS sent a player (research/captures/bds-dragon-home.txt;
//! `changeDimensionWithCredits` 140759540, research/dragon-fight-decompile.md 9): the first time the
//! credits are shown and the trip waits for the client to say they are over; then, as BDS's
//! respawn, its health told, the place it will rise searched for, and a dimension change to the
//! overworld naming none; once the client is there, the place found, and the player put on it.
// TODO: whether BDS keeps a player's seen credits with it was not read; here they last its stay.

use acacia_proto::packets::{Respawn, SetHealth, ShowCredits};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_data::EYE_HEIGHT;

use crate::components::{InLayer, Layer, NetId, Outgoing, PacketQueues, Position};
use crate::{dimensions, movement};

const STARTED: i32 = 0;
const FINISHED: i32 = 1;
const SEARCHING: u8 = 0;
const READY: u8 = 1;
const HEALTH: i32 = 20;
const UNFOUND: Vec3f = Vec3f { x: 0.0, y: 32767.0, z: 0.0 };

/// A player who has seen the credits.
#[derive(Component)]
pub(crate) struct SawCredits;

/// A player watching the credits.
#[derive(Component)]
pub(crate) struct WatchingCredits;

fn eyes([x, y, z]: [f32; 3]) -> Vec3f {
    Vec3f { x, y: y + EYE_HEIGHT, z }
}

/// A player in the exit portal: the credits the first time, else home at once.
pub(super) fn enter(world: &mut World, player: Entity) {
    if world.get::<WatchingCredits>(player).is_some() {
        return;
    }
    if world.get::<SawCredits>(player).is_some() {
        return go_home(world, player);
    }
    let id = world.get::<NetId>(player).map_or(0, |id| id.0);
    world.entity_mut(player).insert(WatchingCredits);
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.push(&ShowCredits { runtime_entity_id: id, status: STARTED });
    }
}

/// The players whose credits are over go home.
pub(crate) fn credits_over(world: &mut World) {
    let over: Vec<Entity> = world.resource_mut::<PacketQueues>().take::<ShowCredits>().into_iter().filter(|(_, shown)| shown.status == FINISHED).map(|(player, _)| player).collect();
    for player in over {
        if world.entity_mut(player).take::<WatchingCredits>().is_some() {
            world.entity_mut(player).insert(SawCredits);
            go_home(world, player);
        }
    }
}

fn spawn_of(world: &mut World, player: Entity) -> Option<(Entity, [f32; 3])> {
    let home = dimensions::home(world, world.get::<InLayer>(player)?.0);
    let spawn = crate::sleep::spawn_of(world, player, home).flatten().unwrap_or(world.get::<Layer>(home)?.spawn);
    Some((home, spawn))
}

fn go_home(world: &mut World, player: Entity) {
    let Some((home, spawn)) = spawn_of(world, player) else { return };
    crate::vitals::mend_fully(world, player);
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.push(&SetHealth { health: HEALTH });
        out.push(&Respawn { position: eyes(spawn), state: SEARCHING, runtime_entity_id: 0 });
    }
    dimensions::go_home(world, player, home, spawn);
}

/// A player home from the End has arrived: where it rises, and it is put there.
pub(crate) fn risen_home(world: &mut World, player: Entity, feet: [f32; 3]) {
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.push(&Respawn { position: UNFOUND, state: SEARCHING, runtime_entity_id: 0 });
        out.push(&Respawn { position: eyes(feet), state: READY, runtime_entity_id: 0 });
        out.push(&SetHealth { health: HEALTH });
    }
    let yaw = world.get::<Position>(player).map_or(0.0, |position| position.yaw);
    movement::reset(world, player, feet, yaw);
}
