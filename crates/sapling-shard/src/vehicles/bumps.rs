//! Minecarts meeting minecarts, as BDS's did (research/captures/bds-l2carts.txt, bds-p2carts.txt,
//! docs/research/vehicles.md "Carts on carts"): a cart rolling into another stops at its side, and carts
//! within a fifth of a block of each other push apart, each pair twice a tick, the Java game's minecart push
//! (`EntityMinecart.applyEntityCollision`); carts overlapping a little are parted at once; one stops
//! against a block.

use bevy_ecs::prelude::*;

use super::carts::Cart;
use crate::body::Body;
use crate::components::InLayer;

/// The push at a centre distance of a block or less, and the share of its own motion each cart keeps.
const PUSH: f32 = 0.05;
const KEPT: f32 = 0.2;
const NEAREST: f32 = 0.01;
/// How far across a cart reaches for others to push (Java's `grow(0.2, 0, 0.2)`): BDS's carts 1.0 to
/// 1.18 apart pushed each other.
const REACH: f32 = 0.2;
/// Boxes this close count as touching.
const CONTACT: f32 = 1e-4;
/// Overlapping by this much or less, a settled cart is moved out of the other by the whole overlap and
/// stops after that tick's move: BDS's parted at 0.045 to 0.1056, not at 0.165 (bds-p2carts.txt).
const CLEARED: f32 = 0.125;

/// A cart moved out of another this tick, by this much along x and z, after its move.
#[derive(Component, Clone, Copy)]
pub(super) struct Parting(pub [f32; 2]);

fn touch(a: &Body, b: &Body, across: f32) -> bool {
    let ((a_low, a_high), (b_low, b_high)) = (a.bounds(), b.bounds());
    (0..3).all(|axis| {
        let reach = if axis == 1 { CONTACT } else { across };
        a_low[axis] <= b_high[axis] + reach && b_low[axis] <= a_high[axis] + reach
    })
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

/// How far `this` moves out of `other` when their boxes overlap by [`CLEARED`] or less across the axis
/// they overlap least on.
fn parting(this: &Body, other: &Body) -> Option<[f32; 2]> {
    if !touch(this, other, -CONTACT) {
        return None;
    }
    let overlap = |axis: usize| (this.size[0] + other.size[0]) / 2.0 - (other.at[axis] - this.at[axis]).abs();
    let axis = if overlap(0) <= overlap(2) { 0 } else { 2 };
    let by = overlap(axis);
    let away = if other.at[axis] > this.at[axis] { -by } else { by };
    (by <= CLEARED).then_some(if axis == 0 { [away, 0.0] } else { [0.0, away] })
}

/// Each pair of carts in reach of each other in a world pushes apart, once from each side, and a settled
/// cart overlapping another a little is set to part from it; a cart in its first tick neither pushes nor
/// is pushed (BDS's summoned one moved a tick after its AddEntity, bds-l2carts.txt).
pub(super) fn push(world: &mut World) {
    let carts: Vec<(Entity, Entity, Body, bool)> = world
        .query_filtered::<(Entity, &InLayer, &Body, Has<super::detectors::Settled>), With<Cart>>()
        .iter(world)
        .map(|(cart, layer, body, settled)| (cart, layer.0, *body, settled))
        .collect();
    let mut bodies: Vec<Body> = carts.iter().map(|(_, _, body, _)| *body).collect();
    let mut parted = vec![[0.0_f32; 2]; carts.len()];
    for i in 0..carts.len() {
        for j in i + 1..carts.len() {
            if carts[i].1 != carts[j].1 {
                continue;
            }
            for (this, other) in [(i, j), (j, i)] {
                if let Some(by) = parting(&carts[this].2, &carts[other].2).filter(|_| carts[this].3) {
                    parted[this] = [parted[this][0] + by[0], parted[this][1] + by[1]];
                }
            }
            if !carts[i].3 || !carts[j].3 || !touch(&bodies[i], &bodies[j], REACH) {
                continue;
            }
            let (left, right) = bodies.split_at_mut(j);
            collide(&mut left[i], &mut right[0]);
            collide(&mut right[0], &mut left[i]);
        }
    }
    for (((cart, _, before, _), after), by) in carts.into_iter().zip(bodies).zip(parted) {
        if before.motion != after.motion
            && let Some(mut body) = world.get_mut::<Body>(cart)
        {
            body.motion = after.motion;
        }
        if by != [0.0; 2] {
            world.entity_mut(cart).insert(Parting(by));
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
    others.into_iter().find(|other| touch(&moved, other, CONTACT) && !touch(from, other, CONTACT)).map(|other| {
        let (dx, dz) = (to[0] - from.at[0], to[2] - from.at[2]);
        // How far it goes before its box meets the other's, along the larger of its two axes.
        let (step, gap) = if dx.abs() >= dz.abs() { (dx.abs(), (other.at[0] - from.at[0]).abs() - width) } else { (dz.abs(), (other.at[2] - from.at[2]).abs() - width) };
        let share = if step > 0.0 { (gap / step).clamp(0.0, 1.0) } else { 0.0 };
        [from.at[0] + dx * share, to[1], from.at[2] + dz * share]
    })
}

/// Where a cart rolling from `from` to `to` stops against the blocks, its box's bottom at `bottom` as it
/// moves, and its motion with what ran into one zeroed: BDS's stopped at 12.51 against a stone at 13, on a
/// flat rail and up a slope (bds-l2carts.txt, bds-p2carts.txt).
pub(super) fn walled(level: &sapling_world::World, from: &Body, to: [f32; 3], bottom: f32, motion: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let mut moved = *from;
    moved.at[1] = bottom;
    moved.shove(level, [to[0] - from.at[0], 0.0, to[2] - from.at[2]]);
    let mut motion = motion;
    for axis in [0, 2] {
        if (moved.at[axis] - to[axis]).abs() > 1e-5 {
            motion[axis] = 0.0;
        }
    }
    ([moved.at[0], to[1], moved.at[2]], motion)
}
