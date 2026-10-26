//! Where a rider steps off to, as BDS puts a passenger leaving a vehicle with the default
//! `dismount_mode` (matched live for a pig and a boat: acacia's docs/research/riding-fishing-elytra.md):
//! the first free spot of 24 around the vehicle's cell, by the way it moves, at the seat's x and z.
// TODO: `on_top_center` vehicles and the blocks BDS ignores for an exit are not modelled.

use acacia_physics::{Aabb, WorldView};
use bevy_ecs::prelude::*;

use super::{kind_of, seats, vehicle_wire};
use crate::body::Body;
use crate::components::{InLayer, Layer, Position};
use crate::entities::Kind;
use crate::terrain::physics_view;

const PLAYER_SIZE: [f32; 2] = [0.6, 1.8];
/// Squared speed across, a tick, up to which the vehicle counts as standing still.
const STILL: f32 = 9.801e-5;
/// Feet are put this far above the floor found.
const LIFT: f32 = 0.001;

/// The eight offsets across, in the order they are tried: from the way the vehicle moves.
fn around([dx, dz]: [f32; 2]) -> [[f32; 2]; 8] {
    let f = if dx * dx + dz * dz <= STILL {
        [-1.0, 0.0]
    } else if dx.abs() > dz.abs() {
        [dx.signum(), 0.0]
    } else {
        [0.0, dz.signum()]
    };
    let s = [-f[1], f[0]];
    let add = |a: [f32; 2], b: [f32; 2], k: f32| [a[0] + k * b[0], a[1] + k * b[1]];
    let neg = |a: [f32; 2]| [-a[0], -a[1]];
    [s, neg(s), add(s, f, -1.0), add(neg(s), f, -1.0), add(s, f, 1.0), add(f, s, -1.0), neg(f), f]
}

/// The height of the cell's highest collision box above its base.
fn top(view: &impl WorldView, cell: [i32; 3]) -> Option<f32> {
    let mut boxes = Vec::new();
    view.block_collisions(cell, &mut boxes);
    boxes.iter().map(|b| b.max[1]).reduce(f32::max)
}

/// The floor in the cell of `at` when a rider of `size` fits standing on it there.
fn free_floor(view: &impl WorldView, at: [f32; 3], [width, height]: [f32; 2]) -> Option<f32> {
    let cell = at.map(|c| c.floor() as i32);
    let floor = match top(view, cell) {
        Some(top) if top > 0.0 && top < 1.0 => top,
        Some(_) => return None,
        None => Some(top(view, [cell[0], cell[1] - 1, cell[2]])? - 1.0).filter(|below| *below >= 0.0)?,
    };
    let half = width.min(1.0) / 2.0;
    let body = Aabb { min: [at[0] - half, at[1] + floor, at[2] - half], max: [at[0] + half, at[1] + floor + height, at[2] + half] };
    let mut hits = Vec::new();
    view.collisions(&body, &mut hits);
    hits.is_empty().then_some(floor)
}

/// The feet a rider of `size` in `seat` (world) steps off to beside a vehicle at `vehicle` moving by `motion`.
fn exit_feet(view: &impl WorldView, vehicle: [f32; 3], seat: [f32; 3], motion: [f32; 2], size: [f32; 2]) -> Option<[f32; 3]> {
    let centre = [vehicle[0].floor() + 0.5, vehicle[1].floor(), vehicle[2].floor() + 0.5];
    for dy in [0.0, 1.0, -1.0] {
        for [hx, hz] in around(motion) {
            if let Some(floor) = free_floor(view, [centre[0] + hx, centre[1] + dy, centre[2] + hz], size) {
                return Some([seat[0] + hx, vehicle[1].floor() + floor + dy + LIFT, seat[2] + hz]);
            }
        }
    }
    None
}

/// Puts a rider that just left `vehicle` where it steps off to; it stays where it was where no
/// spot is free. A player is told, as BDS corrects one taken off by a command.
pub(super) fn step_off(world: &mut World, rider: Entity, vehicle: Entity) {
    let (Some((wire, yaw)), Some(motion), Some(layer)) = (vehicle_wire(world, vehicle), world.get::<Body>(vehicle).map(|body| body.motion), world.get::<InLayer>(vehicle).map(|layer| layer.0)) else { return };
    let Some(kind) = kind_of(world, rider) else { return };
    let seat = world.get::<Kind>(vehicle).and_then(|vehicle| super::layout(&vehicle.0)?.seat(1, 0)).map_or([0.0; 3], |seat| seats::offset(&seat, &kind, yaw));
    let size = world.get::<Body>(rider).map_or(PLAYER_SIZE, |body| body.size);
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let seated = [wire[0] + seat[0], wire[1], wire[2] + seat[2]];
    let Some(feet) = physics_view(level, |view| exit_feet(view, wire, seated, [motion[0], motion[2]], size)) else { return };
    if world.get::<Position>(rider).is_some() {
        crate::movement::push(world, rider, feet);
    } else if let Some(mut body) = world.get_mut::<Body>(rider) {
        body.at = feet;
    }
}

#[cfg(test)]
mod tests {
    use sapling_world::World as Level;

    use super::*;

    #[test]
    fn a_rider_steps_off_to_the_side_of_a_still_vehicle() {
        let mut level = Level::void();
        let stone = level.block_id("minecraft:stone").unwrap();
        level.fill([-4, 63, -4], [4, 63, 4], stone);
        let feet = physics_view(&level, |view| exit_feet(view, [0.5, 64.35, 0.5], [0.5, 64.35, 0.5], [0.0; 2], PLAYER_SIZE)).unwrap();
        assert_eq!(feet, [0.5, 64.0 + LIFT, -0.5], "left of a vehicle facing -x: -z");
        let walled = physics_view(&level, |view| exit_feet(view, [0.5, 64.35, 0.5], [0.5, 64.35, 0.5], [0.3, 0.0], PLAYER_SIZE)).unwrap();
        assert_eq!(walled, [0.5, 64.0 + LIFT, 1.5], "moving +x: +z first");
    }
}
