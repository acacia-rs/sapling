//! The server putting a player somewhere, told to its client as BDS tells it.

use acacia_proto::packets::{CorrectPlayerMovePrediction, CorrectPlayerMovePredictionPredictionType as Kind, MovePlayer, MovePlayerMode, MovePlayerTeleport, MovePlayerTeleportCause};
use acacia_proto::types::LegacyEntityType;
use bevy_ecs::prelude::*;

use super::{LANDING_TICKS, Motion, SETBACK_TICKS, TELEPORT_SOURCE, eyes, move_player};
use crate::components::{NetId, Outgoing, Position, Tick};
use crate::simulation::Simulated;

/// Puts a player's feet at `feet` and tells its client, as BDS's `/tp` does. Viewers learn of it
/// from [`super::announce`].
pub(crate) fn teleport(world: &mut World, entity: Entity, feet: [f32; 3]) {
    teleport_as(world, entity, feet, MovePlayerTeleportCause::Command, &[TELEPORT_SOURCE]);
}

/// [`teleport`] told as one MovePlayer per source entity type, each with `cause`.
pub(crate) fn teleport_as(world: &mut World, entity: Entity, feet: [f32; 3], cause: MovePlayerTeleportCause, sources: &[i64]) {
    let tick = world.resource::<Tick>().0;
    let Ok((id, mut position, mut motion, mut out)) = world.query::<(&NetId, &mut Position, &mut Motion, &mut Outgoing)>().get_mut(world, entity) else { return };
    (position.feet, position.on_ground) = (feet, true);
    motion.unconfirmed_teleport = Some(tick + SETBACK_TICKS);
    for source in sources {
        let moved = move_player(id.0, &position, Some(cause.clone()), motion.input_tick);
        out.push(&MovePlayer { teleport: moved.teleport.map(|teleport| MovePlayerTeleport { source_entity_type: LegacyEntityType::from_raw(*source), ..teleport }), ..moved });
    }
}

/// Puts a player's feet at `feet` facing `yaw` and tells its client with MovePlayer mode Reset,
/// as BDS places one that came through a portal (research/captures/bds-portal-light.txt).
pub(crate) fn reset(world: &mut World, entity: Entity, feet: [f32; 3], yaw: f32) {
    if let Some(mut simulated) = world.get_mut::<Simulated>(entity) {
        simulated.restart();
    }
    let Ok((id, mut position, motion, mut out)) = world.query::<(&NetId, &mut Position, &Motion, &mut Outgoing)>().get_mut(world, entity) else { return };
    (position.feet, position.yaw, position.on_ground) = (feet, yaw, true);
    let moved = MovePlayer { mode: MovePlayerMode::Reset, ..move_player(id.0, &position, None, motion.input_tick) };
    out.push(&moved);
}

/// Puts a player's feet at `feet` where something pushed it, telling its client as BDS told one
/// a piston pushed (research/captures/bds-pushed.txt): its prediction is corrected.
pub(crate) fn push(world: &mut World, entity: Entity, feet: [f32; 3]) {
    let tick = world.resource::<Tick>().0;
    if let Some(mut motion) = world.get_mut::<Motion>(entity) {
        motion.unconfirmed_teleport = Some(tick + SETBACK_TICKS);
    }
    correct(world, entity, feet);
}

/// Puts a player's feet at `feet` by correcting its prediction, as BDS does once a client is
/// through a dimension change, and takes its next input whatever it says (dimensions.rs).
pub(crate) fn correct(world: &mut World, entity: Entity, feet: [f32; 3]) {
    // The fall a body standing still has in a tick, as BDS sent it.
    correction(world, entity, feet, -0.0784);
}

/// [`correct`] with no fall in it, as BDS corrects a player it sends through an end portal.
pub(crate) fn correct_unmoving(world: &mut World, entity: Entity, feet: [f32; 3]) {
    correction(world, entity, feet, 0.0);
}

fn correction(world: &mut World, entity: Entity, feet: [f32; 3], fall: f32) {
    if let Some(mut simulated) = world.get_mut::<Simulated>(entity) {
        simulated.adopt(feet, [0.0, fall, 0.0], true);
    }
    let Ok((mut position, motion, mut out)) = world.query::<(&mut Position, &Motion, &mut Outgoing)>().get_mut(world, entity) else { return };
    (position.feet, position.on_ground) = (feet, true);
    out.push(&prediction(&position, [0.0, fall, 0.0], motion.input_tick));
}

/// The client's prediction put right: at `position` moving by `delta` at the end of input `tick`.
pub(crate) fn prediction(position: &Position, [x, y, z]: [f32; 3], tick: u64) -> CorrectPlayerMovePrediction {
    let delta = acacia_proto::types::Vec3f { x, y, z };
    let rotation = acacia_proto::types::Vec2f { x: 0.0, z: 0.0 };
    CorrectPlayerMovePrediction { prediction_type: Kind::Player, position: eyes(position), delta, rotation, angular_velocity: None, on_ground: position.on_ground, tick }
}

/// [`teleport`] onto ground the client may not have yet (another world's spawn): until it
/// stands there, a client that fell meanwhile is put back, as BDS's own simulation would
/// correct it.
pub(crate) fn teleport_onto_ground(world: &mut World, entity: Entity, feet: [f32; 3]) {
    teleport(world, entity, feet);
    let until = world.resource::<Tick>().0 + LANDING_TICKS;
    if let Some(mut motion) = world.get_mut::<Motion>(entity) {
        motion.landing = Some((feet, until));
    }
}
