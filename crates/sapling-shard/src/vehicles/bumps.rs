//! Minecarts meeting minecarts, as BDS's did (research/captures/bds-l2carts.txt, docs/research/vehicles.md
//! "Carts on carts"): a cart rolling into another stops at its side, and carts whose boxes touch push each
//! other apart, each pair twice a tick, the Java game's minecart push (`EntityMinecart.applyEntityCollision`);
//! one on a flat rail stops against a block.

use bevy_ecs::prelude::*;

use super::carts::Cart;
use crate::body::Body;
use crate::components::InLayer;

/// The push at a centre distance of a block or less, and the share of its own motion each cart keeps.
const PUSH: f32 = 0.05;
const KEPT: f32 = 0.2;
const NEAREST: f32 = 0.01;

/// Boxes this close count as touching: a cart stopped at another's side is pushed off it.
const CONTACT: f32 = 1e-4;

fn touch(a: &Body, b: &Body) -> bool {
    let ((a_low, a_high), (b_low, b_high)) = (a.bounds(), b.bounds());
    (0..3).all(|axis| a_low[axis] <= b_high[axis] + CONTACT && b_low[axis] <= a_high[axis] + CONTACT)
}

/// `this` pushed off `other`: each keeps a fifth of its motion, takes their mean and is pushed apart along
/// the line between them, by [`PUSH`] up to a block and less past it.
fn collide(this: &mut Body, other: &mut Body) {
    let (dx, dz) = (other.at[0] - this.at[0], other.at[2] - this.at[2]);
    let far = dx.hypot(dz);
    if far < NEAREST {
        return;
    }
    let force = (1.0 / far).min(1.0) * PUSH / far;
    let (px, pz) = (dx * force, dz * force);
    let (mx, mz) = ((this.motion[0] + other.motion[0]) / 2.0, (this.motion[2] + other.motion[2]) / 2.0);
    (this.motion[0], this.motion[2]) = (this.motion[0] * KEPT + mx - px, this.motion[2] * KEPT + mz - pz);
    (other.motion[0], other.motion[2]) = (other.motion[0] * KEPT + mx + px, other.motion[2] * KEPT + mz + pz);
}

/// Each pair of touching carts in a world pushes apart, once from each side.
pub(super) fn push(world: &mut World) {
    let carts: Vec<(Entity, Entity, Body)> = world.query_filtered::<(Entity, &InLayer, &Body), With<Cart>>().iter(world).map(|(cart, layer, body)| (cart, layer.0, *body)).collect();
    let mut bodies: Vec<Body> = carts.iter().map(|(_, _, body)| *body).collect();
    for i in 0..carts.len() {
        for j in i + 1..carts.len() {
            if carts[i].1 != carts[j].1 || !touch(&bodies[i], &bodies[j]) {
                continue;
            }
            let (left, right) = bodies.split_at_mut(j);
            collide(&mut left[i], &mut right[0]);
            collide(&mut right[0], &mut left[i]);
        }
    }
    for ((cart, _, before), after) in carts.into_iter().zip(bodies) {
        if before.motion != after.motion
            && let Some(mut body) = world.get_mut::<Body>(cart)
        {
            body.motion = after.motion;
        }
    }
}

/// Where a cart moving from `from` to `to` stops: at the side of the first other cart in `layer` its box
/// would run into, with no motion left, or `None` where nothing is in its way (or it already touched one).
pub(super) fn blocked(world: &mut World, cart: Entity, layer: Entity, from: &Body, to: [f32; 3]) -> Option<[f32; 3]> {
    let mut moved = *from;
    moved.at = to;
    let others: Vec<Body> = world.query_filtered::<(Entity, &InLayer, &Body), With<Cart>>().iter(world).filter(|(other, in_layer, _)| *other != cart && in_layer.0 == layer).map(|(_, _, body)| *body).collect();
    let width = from.size[0];
    others.into_iter().find(|other| touch(&moved, other) && !touch(from, other)).map(|other| {
        let (dx, dz) = (to[0] - from.at[0], to[2] - from.at[2]);
        // How far it goes before its box meets the other's, along the larger of its two axes.
        let (step, gap) = if dx.abs() >= dz.abs() { (dx.abs(), (other.at[0] - from.at[0]).abs() - width) } else { (dz.abs(), (other.at[2] - from.at[2]).abs() - width) };
        let share = if step > 0.0 { (gap / step).clamp(0.0, 1.0) } else { 0.0 };
        [from.at[0] + dx * share, to[1], from.at[2] + dz * share]
    })
}

/// Where a cart rolling from `from` to `to` along a flat rail stops against the blocks, and its motion
/// with what ran into one zeroed: BDS's stopped at 12.51 against a stone at 13 (bds-l2carts.txt).
// TODO: slopes are left unchecked, their own rise reaching into the block they climb onto.
pub(super) fn walled(level: &sapling_world::World, from: &Body, to: [f32; 3], motion: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let mut moved = *from;
    moved.at[1] = to[1];
    moved.shove(level, [to[0] - from.at[0], 0.0, to[2] - from.at[2]]);
    let mut motion = motion;
    for axis in [0, 2] {
        if (moved.at[axis] - to[axis]).abs() > 1e-5 {
            motion[axis] = 0.0;
        }
    }
    ([moved.at[0], to[1], moved.at[2]], motion)
}
