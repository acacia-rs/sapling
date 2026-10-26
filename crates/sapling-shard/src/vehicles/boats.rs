//! Boats: placed on top of the block a boat item is used on, facing across the player, and moved
//! by acacia's model of BDS's boat (acacia-physics `boat_tick`: friction, paddles, buoyancy and
//! waves) whenever no driver's client moves it (riding/drive.rs).

use acacia_physics::{BoatState, boat_tick};
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};

use super::{is_crowded, spawn};
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId};
use crate::entities;
use crate::sync::Told;
use crate::terrain::physics_view;

/// The wire puts a boat this far above its box's bottom.
const LIFT: f32 = 0.375;
/// A boat faces this far round from the player who placed it.
const ACROSS: f32 = 90.0;

/// A boat's motion, as acacia's model keeps it.
#[derive(Component, Clone, Copy)]
pub(super) struct Boat(BoatState);

/// Gives a boat just put at `wire` facing `yaw` its motion.
pub(super) fn fit(world: &mut World, boat: Entity, wire: [f32; 3], yaw: f32) {
    world.entity_mut(boat).insert(Boat(BoatState::new(wire, yaw)));
}

/// Places a boat of `kind` in `wood` on top of the block at `pos`, where the click landed `at`
/// within it, facing round from the player's `yaw`; false off the top face, under a block, or
/// where a mob or another vehicle is in the way (BDS `BoatItem::_useOn`).
#[allow(clippy::too_many_arguments)]
pub(super) fn place(world: &mut World, layer: Entity, pos: BlockPos, face: Face, at: [f32; 3], yaw: f32, kind: &str, wood: i32) -> bool {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return false };
    let [x, y, z] = pos;
    let above_is_air = level.in_height(y + 1) && level.registry().get(level.block(x, y + 1, z)).is_some_and(|state| state.is_air());
    if face != Face::Up || !above_is_air {
        return false;
    }
    let wire = [x as f32 + at[0].clamp(0.0, 1.0), (y + 1) as f32, z as f32 + at[2].clamp(0.0, 1.0)];
    let feet = [wire[0], wire[1] - LIFT, wire[2]];
    let Some(size) = sapling_data::mob_size(kind) else { return false };
    if is_crowded(world, layer, Body::new(feet, [0.0; 3], size).bounds()) {
        return false;
    }
    spawn(world, layer, kind, feet, yaw + ACROSS, wood, None).inspect(|boat| tell_placed(world, *boat, layer, wire)).is_some()
}

/// The splash BDS made as a boat was placed on water, a little under the wire.
fn tell_placed(world: &mut World, boat: Entity, layer: Entity, [x, y, z]: [f32; 3]) {
    const BELOW: f32 = 0.1475;
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let cell = [x, y - BELOW, z].map(|c| c.floor() as i32);
    let is_water = |id: u32| level.registry().get(id).is_some_and(|state| state.name.ends_with("water"));
    if !(is_water(level.block(cell[0], cell[1], cell[2])) || is_water(level.liquid(cell[0], cell[1], cell[2]))) {
        return;
    }
    let Some(id) = world.get::<NetId>(boat).map(|id| id.0) else { return };
    let splash = crate::riding::sound_packet("splash", id, super::BOAT, [x, y - BELOW, z]);
    entities::tell_viewers(world, boat, layer, cell, vec![splash]);
}

/// Moves each boat no driver's client moves a tick.
pub(super) fn tick(world: &mut World) {
    let boats: Vec<(Entity, Entity, u64, Boat, Body)> = world.query::<(Entity, &InLayer, &NetId, &Boat, &Body)>().iter(world).map(|(boat, layer, id, state, body)| (boat, layer.0, id.0, *state, *body)).collect();
    for (boat, layer, id, Boat(mut state), body) in boats {
        let wire = [body.at[0], body.at[1] + LIFT, body.at[2]];
        if crate::riding::is_driven(world, boat) {
            let yaw = world.get::<Told>(boat).map_or(state.yaw, |told| told.facing()[0]);
            (state.pos, state.vel, state.yaw) = (wire, body.motion, yaw);
            world.entity_mut(boat).insert(Boat(state));
            continue;
        }
        state.pos = wire;
        let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { continue };
        physics_view(level, |view| boat_tick(&mut state, [0.0; 2], view));
        let [x, y, z] = state.pos;
        let feet = [x, y - LIFT, z];
        world.entity_mut(boat).insert(Boat(state));
        if let Some(mut moved) = world.get_mut::<Body>(boat) {
            (moved.at, moved.motion, moved.on_ground) = (feet, state.vel, state.on_ground);
        }
        let Some(mut told) = world.get_mut::<Told>(boat) else { continue };
        told.face(state.yaw, state.yaw);
        let packets = told.update(id, state.pos, state.vel, state.on_ground);
        entities::tell_viewers(world, boat, layer, feet.map(|c| c.floor() as i32), packets);
    }
}
