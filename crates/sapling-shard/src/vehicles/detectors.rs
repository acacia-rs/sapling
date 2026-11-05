//! Detector rails (BDS `DetectorRailBlock`: entityInside 148a1e630, checkPressed 148a1e7b0, tick
//! 148a1ee00; docs/research/vehicles.md "Minecarts"): a cart in a rail's cell at the end of a tick
//! presses it at the start of the next, and a pressed rail is checked again every 20 ticks from the
//! press, let go at the first check that finds no cart.

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::carts::Cart;
use super::rails::{self, RailKind};
use crate::body::Body;
use crate::components::{InLayer, Layer, Tick};

const RECHECK: u64 = 20;
/// The part of its cell a cart must reach to press a rail (`checkPressed`'s box).
const FELT_MIN: [f32; 3] = [0.2, 0.0, 0.2];
const FELT_MAX: [f32; 3] = [0.8, 0.8, 0.8];

/// Pressed detector rails and the tick each is checked again.
#[derive(Resource, Default)]
struct Checks(HashMap<(Entity, BlockPos), u64>);

/// A cart that has ended a tick somewhere; a cart summoned this tick is in no rail's cell yet.
#[derive(Component)]
pub(super) struct Settled;

/// Presses the detector rails settled carts are in, then checks those due.
pub(super) fn tick(world: &mut World) {
    let carts: Vec<(Entity, Body)> = world.query_filtered::<(&InLayer, &Body), (With<Cart>, With<Settled>)>().iter(world).map(|(layer, body)| (layer.0, *body)).collect();
    for (layer, body) in carts {
        let (low, high) = body.bounds();
        let span = |axis: usize| low[axis].floor() as i32..=high[axis].floor() as i32;
        for pos in span(0).flat_map(|x| span(1).flat_map(move |y| span(2).map(move |z| [x, y, z]))) {
            if world.get::<Layer>(layer).and_then(|level| rails::rail_at(&level.world, pos)).is_some_and(|rail| rail.kind == RailKind::Detector && !rail.on) {
                check(world, layer, pos);
            }
        }
    }
    if world.get_resource::<Checks>().is_none() {
        return;
    }
    let (region, now) = (crate::ticking::Region::of(world), world.resource::<Tick>().0);
    let checks = world.resource::<Checks>();
    let due: Vec<(Entity, BlockPos)> = checks.0.iter().filter(|&(&(layer, pos), &when)| region.runs_due(world, when, now, layer, pos)).map(|(at, _)| *at).collect();
    for (layer, pos) in due {
        check(world, layer, pos);
    }
}

/// Whether a cart's box reaches into the part of `pos` that presses it.
fn is_pressed(world: &mut World, layer: Entity, [x, y, z]: BlockPos) -> bool {
    let cell = [x as f32, y as f32, z as f32];
    let mut carts = world.query_filtered::<(&InLayer, &Body), With<Cart>>();
    carts.iter(world).filter(|(on, _)| on.0 == layer).any(|(_, body)| {
        let (low, high) = body.bounds();
        (0..3).all(|axis| low[axis] < cell[axis] + FELT_MAX[axis] && high[axis] > cell[axis] + FELT_MIN[axis])
    })
}

/// BDS `checkPressed`: the rail at `pos` pressed while a cart is on it, and while pressed checked
/// again 20 ticks on.
fn check(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(rail) = world.get::<Layer>(layer).and_then(|level| rails::rail_at(&level.world, pos)).filter(|rail| rail.kind == RailKind::Detector) else {
        world.get_resource_or_insert_with(Checks::default).0.remove(&(layer, pos));
        return;
    };
    let pressed = is_pressed(world, layer, pos);
    if pressed != rail.on {
        set_pressed(world, layer, pos, pressed);
    }
    let when = world.resource::<Tick>().0 + RECHECK;
    let mut checks = world.get_resource_or_insert_with(Checks::default);
    match pressed {
        true => drop(checks.0.insert((layer, pos), when)),
        false => drop(checks.0.remove(&(layer, pos))),
    }
}

fn set_pressed(world: &mut World, layer: Entity, [x, y, z]: BlockPos, on: bool) {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    if let Some(id) = crate::blocks::with_property(level.registry(), level.block(x, y, z), "rail_data_bit", if on { "1" } else { "0" }) {
        crate::blocks::set(world, layer, [x, y, z], id);
    }
}
