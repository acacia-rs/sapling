//! A ghast, as BDS's flew and shot (research/captures/bds-ghast-findings.md): it floats towards a
//! whole-block spot up to 10 off across and 7 up or down, pushed 0.03 a tick (`Physics::FLYER`),
//! and picks another within a block of it or after 2 to 7 seconds. With prey in sight it opens its
//! mouth with `warn` 20 ticks after taking it, shoots 21 after that, and warns again 60 after each
//! shot (`fireballs`).
//! A spot its box would not reach, there or on the straight way, is given up and another picked
//! the tick after; slowness slows its push as it slows a walker (bds-ghast.txt: held at 0, it hung still).

use acacia_proto::types::MetadataFlags1;
use bevy_ecs::prelude::*;
use sapling_world::World as Level;

use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{Layer, Tick};
use crate::{entities, late_sounds};
use super::{fireballs, hunting};

const ACROSS: f32 = 10.0;
const UP_OR_DOWN: f32 = 7.0;
const PUSH: f32 = 0.03;
const REACHED: f32 = 1.0;
const FLOATS: [u64; 2] = [40, 140];
/// Ticks from taking its prey to its warning, from the warning to its mouth's closing and to the
/// shot, and from a shot to the next warning.
const WARNS: u64 = 20;
const CLOSES: u64 = 19;
const SHOOTS: u64 = 21;
const AGAIN: u64 = 60;
/// Its sounds come from this much of its height up.
const MIDDLE: f32 = 0.5;

#[derive(Component, Default)]
pub(super) struct Ghast {
    spot: Option<[f32; 3]>,
    until: u64,
    /// The tick of its next warning and, once it warned, of the shot.
    warns: Option<u64>,
    shoots: Option<u64>,
}

pub(super) fn arm(world: &mut World, entity: Entity, kind: &str) {
    if sapling_data::mob_traits(kind).hunts.is_some_and(|hunts| hunts.fights == Some("fireballs")) {
        world.entity_mut(entity).insert(Ghast::default());
    }
}

/// Before its move: it floats on, and with prey at `prey` warns and shoots when they are due.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body, prey: Option<[f32; 3]>) -> bool {
    let now = world.resource::<Tick>().0;
    let Some(ghast) = world.get::<Ghast>(entity) else { return false };
    let (spot, until, warns, shoots) = (ghast.spot, ghast.until, ghast.warns, ghast.shoots);
    let near = spot.is_some_and(|spot| (0..3).map(|axis| (spot[axis] - body.at[axis]).powi(2)).sum::<f32>().sqrt() < REACHED);
    let spot = match spot.filter(|_| !near && now < until) {
        Some(spot) => spot,
        None => {
            let mut dice = world.get_resource_or_insert_with(Dice::default);
            let mut off = |reach: f32| ((dice.fraction() * 2.0 - 1.0) * reach).round();
            let picked = [body.at[0].floor() + off(ACROSS), body.at[1].floor() + off(UP_OR_DOWN), body.at[2].floor() + off(ACROSS)];
            let lasts = FLOATS[0] + dice.roll((FLOATS[1] - FLOATS[0]) as u32 + 1) as u64;
            let reached = world.get::<Layer>(layer).is_some_and(|level| reaches(&level.world, body, picked));
            if let Some(mut ghast) = world.get_mut::<Ghast>(entity) {
                (ghast.spot, ghast.until) = (reached.then_some(picked), now + lasts);
            }
            if !reached {
                return steer_shots(world, entity, layer, id, kind, body, prey, warns, shoots);
            }
            picked
        }
    };
    let towards: [f32; 3] = std::array::from_fn(|axis| spot[axis] - body.at[axis]);
    let long = towards.iter().map(|c| c * c).sum::<f32>().sqrt();
    let push = PUSH * super::afflictions::pace(world, entity);
    if long > 1e-4 && push > 0.0 {
        if let Some(mut moving) = world.get_mut::<Body>(entity) {
            moving.fly(towards.map(|c| c / long * push));
        }
    }
    steer_shots(world, entity, layer, id, kind, body, prey, warns, shoots)
}

/// Whether a ghast of `body` floats to `spot` unhindered: its box free there and at each block
/// of the straight way (the pack's `float_wander` `must_reach`, as the Java game's `GhastMoveControl.canReach`).
fn reaches(level: &Level, body: &Body, spot: [f32; 3]) -> bool {
    let way: [f32; 3] = std::array::from_fn(|axis| spot[axis] - body.at[axis]);
    let steps = way.iter().map(|c| c * c).sum::<f32>().sqrt().ceil().max(1.0) as usize;
    (1..=steps).all(|step| Body::new(std::array::from_fn(|axis| body.at[axis] + way[axis] * step as f32 / steps as f32), [0.0; 3], body.size).is_free(level))
}

/// With prey at `prey`, the ghast warns and shoots when they are due.
#[allow(clippy::too_many_arguments)]
fn steer_shots(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body, prey: Option<[f32; 3]>, warns: Option<u64>, shoots: Option<u64>) -> bool {
    let now = world.resource::<Tick>().0;
    let cell = body.at.map(|c| c.floor() as i32);
    let mouth = |world: &mut World, open: bool| {
        let flags = entities::mob_flags(world, entity, kind, MetadataFlags1::CHARGED, open);
        entities::tell_viewers(world, entity, layer, cell, vec![sapling_net::encode(&sapling_data::mob_charge(id, flags, open))]);
    };
    let (warns, shoots) = match (prey, warns, shoots) {
        (None, _, shoots) => {
            if shoots.is_some() {
                mouth(world, false);
            }
            (None, None)
        }
        (Some(_), None, None) => (Some(now + WARNS), None),
        (Some(_), Some(due), None) if now >= due => {
            mouth(world, true);
            late_sounds::play_later(world, 0, layer, cell, hunting::sound("warn", id, kind, body, MIDDLE));
            (None, Some(now + SHOOTS))
        }
        (Some(feet), None, Some(due)) => {
            if now + SHOOTS - CLOSES == due {
                mouth(world, false);
            }
            if now >= due {
                fireballs::fire(world, layer, entity, kind, body, feet);
                late_sounds::play_later(world, 0, layer, cell, hunting::sound("shoot", id, kind, body, MIDDLE));
                (Some(now + AGAIN), None)
            } else {
                (None, Some(due))
            }
        }
        (_, warns, shoots) => (warns, shoots),
    };
    if let Some(mut ghast) = world.get_mut::<Ghast>(entity) {
        (ghast.warns, ghast.shoots) = (warns, shoots);
    }
    true
}
