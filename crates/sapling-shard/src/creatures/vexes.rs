//! Vexes, as an evoker's flew on BDS (research/captures/bds-evoker-findings.md), by the Java game's
//! goals: through blocks (`Physics::GHOST`), pushed 0.05 a tick towards a charge's end and a
//! quarter of that towards a spot up to 7 off where it was summoned; a tick in 7 one with prey
//! more than 2 off charges at its eyes and hits it where their boxes meet. An evoker's vexes
//! start to waste 30 to 119 seconds after they come, 1 a second.
// TODO: a vex re-sent its target every tick with `mob.warning` on BDS, which is not copied; one
// spawned by command did not charge in 30 seconds there (bds-vex.txt), and does here.

use acacia_proto::packets::{EntityEvent, EntityEventEventId};
use acacia_proto::types::MetadataFlags1;
use bevy_ecs::prelude::*;

use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Layer, Position, Tick};
use crate::{entities, late_sounds, vitals};
use super::health::Cause;
use super::{harm, hunting};

pub(super) const KIND: &str = "minecraft:vex";
const PLAYER: [f32; 2] = [0.6, 1.8];
/// It is pushed this hard a tick, times the speed of what it goes for: a charge or a wander.
const PUSH: f32 = 0.05;
const CHARGES: f32 = 1.0;
const WANDERS: f32 = 0.25;
/// A tick in this many it starts a charge, or a wander; it charges only prey farther than this,
/// and keeps aiming at prey nearer than this.
const ODDS: u32 = 7;
const CHARGES_FROM: f32 = 2.0;
const AIMS_WITHIN: f32 = 3.0;
/// A wander goes up to this far each way from where it was summoned.
const ROAMS: i32 = 7;
/// Its hit throws this hard along its flight and up; its sounds come from its middle.
const THROWS: f32 = 0.4;
const MIDDLE: f32 = 0.5;
/// An evoker's vex wastes from this many ticks after it came, plus up to this many more, 1 each
/// this many ticks.
const LASTS: u64 = 600;
const LASTS_MORE: u32 = 1780;
const WASTES_EVERY: u64 = 20;
/// An evoker summons vexes at these whole-block offsets from it, while fewer than so many are
/// within so far of it.
const SUMMONS: [[i32; 2]; 3] = [[1, 0], [-1, -1], [-1, 1]];
const CAP: usize = 8;
const CAP_REACH: f32 = 16.0;

#[derive(Component, Default)]
pub(super) struct Vex {
    home: Option<[f32; 3]>,
    /// Where it is going and how fast.
    wanted: Option<([f32; 3], f32)>,
    charging: bool,
    wastes: Option<u64>,
}

pub(super) fn arm(world: &mut World, entity: Entity, kind: &str) {
    if kind == KIND {
        world.entity_mut(entity).insert(Vex::default());
    }
}

/// Whether an evoker at `at` in `layer` may summon more: fewer than [`CAP`] vexes are within
/// [`CAP_REACH`] of it.
pub(super) fn may_summon(world: &mut World, layer: Entity, at: [f32; 3]) -> bool {
    let mut vexes = world.query::<(&InLayer, &Body, &entities::Kind)>();
    let near = vexes.iter(world).filter(|(in_layer, body, kind)| in_layer.0 == layer && kind.0 == KIND && (0..3).map(|axis| (body.at[axis] - at[axis]).powi(2)).sum::<f32>() < CAP_REACH * CAP_REACH).count();
    near < CAP
}

/// An evoker with the body `evoker` summons [`SUMMONS`]: a block off it, standing where it stands,
/// kept near it and wasting in time.
pub(super) fn summon(world: &mut World, layer: Entity, evoker: &Body) {
    let now = world.resource::<Tick>().0;
    for [dx, dz] in SUMMONS {
        let feet = [(evoker.at[0].floor() as i32 + dx) as f32, evoker.at[1], (evoker.at[2].floor() as i32 + dz) as f32];
        let Some(vex) = super::spawn(world, layer, KIND, feet, sapling_api::SpawnReason::Summoned).and_then(|id| entities::find(world, id)) else { continue };
        let more = u64::from(world.get_resource_or_insert_with(Dice::default).roll(LASTS_MORE));
        if let Some(mut bound) = world.get_mut::<Vex>(vex) {
            (bound.home, bound.wastes) = (Some(evoker.at), Some(now + LASTS + more));
        }
    }
}

fn charging(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body, on: bool) {
    if let Some(mut vex) = world.get_mut::<Vex>(entity) {
        vex.charging = on;
    }
    let flags = sapling_data::mob_flags_set(id, entities::mob_flags(world, entity, kind, MetadataFlags1::CHARGE_ATTACK, on));
    let cell = body.at.map(|c| c.floor() as i32);
    entities::tell_viewers(world, entity, layer, cell, vec![sapling_net::encode(&flags)]);
    if on {
        late_sounds::play_later(world, 1, layer, cell, hunting::sound("charge", id, kind, body, MIDDLE));
    }
}

fn meets(body: &Body, feet: [f32; 3]) -> bool {
    let across = (body.size[0] + PLAYER[0]) / 2.0;
    (body.at[0] - feet[0]).abs() < across && (body.at[2] - feet[2]).abs() < across && body.at[1] < feet[1] + PLAYER[1] && feet[1] < body.at[1] + body.size[1]
}

/// Before its move, with its prey `prey`: it wastes, hits, charges, wanders and flies.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body, prey: Option<Entity>) -> bool {
    let now = world.resource::<Tick>().0;
    let Some(vex) = world.get::<Vex>(entity) else { return false };
    let (home, mut wanted, was_charging, wastes) = (vex.home.unwrap_or(body.at), vex.wanted, vex.charging, vex.wastes);
    if wastes.is_some_and(|from| now >= from && (now - from).is_multiple_of(WASTES_EVERY)) {
        harm::wound(world, entity, layer, 1.0, Cause::Generic);
    }
    let feet = prey.and_then(|prey| world.get::<Position>(prey)).map(|position| position.feet);
    let eyes = feet.map(|feet| [feet[0], feet[1] + sapling_data::EYE_HEIGHT, feet[2]]);
    let far = eyes.map_or(f32::MAX, |eyes| (0..3).map(|axis| (eyes[axis] - body.at[axis]).powi(2)).sum::<f32>().sqrt());
    match (was_charging, prey.zip(feet).zip(eyes)) {
        (true, Some(((prey, feet), _))) if meets(body, feet) => {
            let harm = sapling_data::mob_traits(kind).hunts.and_then(|hunts| hunting::on_difficulty(world, hunts.attack));
            let along = body.motion[0].hypot(body.motion[2]).max(1e-4);
            if let Some(harm) = harm {
                let swing = EntityEvent { runtime_entity_id: id, event_id: EntityEventEventId::ArmSwing, data: 0, fire_at_position: None };
                entities::tell_viewers(world, entity, layer, body.at.map(|c| c.floor() as i32), vec![sapling_net::encode(&swing)]);
                vitals::bitten(world, prey, harm, kind, [body.motion[0] / along * THROWS, THROWS, body.motion[2] / along * THROWS]);
                late_sounds::play_later(world, 1, layer, body.at.map(|c| c.floor() as i32), hunting::sound("attack", id, kind, body, MIDDLE));
            }
            charging(world, entity, layer, id, kind, body, false);
            wanted = None;
        }
        (true, Some((_, eyes))) if far < AIMS_WITHIN => wanted = Some((eyes, CHARGES)),
        (true, None) => {
            charging(world, entity, layer, id, kind, body, false);
            wanted = None;
        }
        (false, Some((_, eyes))) if wanted.is_none() && far > CHARGES_FROM && world.get_resource_or_insert_with(Dice::default).roll(ODDS) == 0 => {
            charging(world, entity, layer, id, kind, body, true);
            wanted = Some((eyes, CHARGES));
        }
        _ => {}
    }
    let charges = world.get::<Vex>(entity).is_some_and(|vex| vex.charging);
    if !charges && wanted.is_none() && world.get_resource_or_insert_with(Dice::default).roll(ODDS) == 0 {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        let mut off = || dice.roll(2 * ROAMS as u32 + 1) as i32 - ROAMS;
        let spot = [home[0].floor() as i32 + off(), home[1].floor() as i32 + off(), home[2].floor() as i32 + off()];
        let open = world.get::<Layer>(layer).is_some_and(|layer| Body::new(spot.map(|c| c as f32 + 0.5), [0.0; 3], [0.9, 0.9]).is_free(&layer.world));
        if open {
            wanted = Some((spot.map(|c| c as f32 + 0.5), WANDERS));
        }
    }
    if let Some((to, speed)) = wanted {
        let towards: [f32; 3] = std::array::from_fn(|axis| to[axis] - body.at[axis]);
        let long = towards.iter().map(|c| c * c).sum::<f32>().sqrt();
        // Within its own size of it, it stops there, its speed halved, and a charge ends.
        if long < (2.0 * body.size[0] + body.size[1]) / 3.0 {
            wanted = None;
            if let Some(mut moving) = world.get_mut::<Body>(entity) {
                moving.motion = moving.motion.map(|c| c * 0.5);
            }
            if charges {
                charging(world, entity, layer, id, kind, body, false);
            }
        } else if let Some(mut moving) = world.get_mut::<Body>(entity) {
            moving.fly(towards.map(|c| c / long * PUSH * speed));
        }
    }
    if let Some(mut vex) = world.get_mut::<Vex>(entity) {
        vex.wanted = wanted;
    }
    true
}
