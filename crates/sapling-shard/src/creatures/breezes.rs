//! A breeze, as BDS's fought (research/captures/bds-breeze-findings.md): it faces its prey for 16
//! ticks and shoots a wind charge (`charges`); 4 ticks on it leaps near prey 4 to 20 off (`leap`),
//! after readying for 10, or walks out to 4 to 8 off prey nearer, then faces it again.
// TODO: jump boost does not add to its leap, its own bursts do not push it, and it does not turn
// as it faces.

mod leap;

use acacia_proto::packets::SetEntityData;
use acacia_proto::types::{EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataFlags2};
use bevy_ecs::prelude::*;

use crate::blocks::Dice;
use crate::body::Body;
use crate::components::Tick;
use crate::{entities, late_sounds};
use super::{charges, hunting, legs};

/// Ticks it faces its prey before a shot, stands after it, readies a leap, and waits where it
/// neither leaps nor walks; it gives up a walk after this many.
const FACES: u64 = 16;
const AFTER: u64 = 4;
const READIES: u64 = 10;
const WAITS: u64 = 10;
const WALKS_FOR: u64 = 60;
/// The first tick of a leap goes this much higher.
const LIFT: f32 = 0.2655;
/// Nearer prey it walks from, this fast, to this far off it; a walk ends this near its end.
const BACKS_TO: [f32; 2] = [4.0, 8.0];
const SPEED: f32 = 0.48;
const ARRIVED: f32 = 0.6;
/// Its `FlagsExtended` at rest, facing its prey and leaping: bits 24 and 39 as BDS sent them,
/// one above acacia-proto's names for them. A frog's leap is told the same way (frogs.rs).
const REST: i64 = i64::MIN | 1 << 25;
const FACING: i64 = 1 << 24;
pub(super) const LEAPING: i64 = 1 << 39;

#[derive(Clone, Copy, Default)]
enum Stage {
    #[default]
    Idle,
    Facing(u64),
    After(u64),
    /// Since when, and the speed it leaps at, picked as it began.
    Readying(u64, [f32; 3]),
    Leaping(u64),
    Landed,
    Walking([f32; 2], u64),
    Waiting(u64),
}

#[derive(Component, Default)]
pub(super) struct Breeze {
    stage: Stage,
}

pub(super) fn arm(world: &mut World, entity: Entity, kind: &str) {
    if sapling_data::mob_traits(kind).hunts.is_some_and(|hunts| hunts.fights == Some("leaps")) {
        world.entity_mut(entity).insert(Breeze::default());
    }
}

pub(super) fn flags(id: u64, set: i64) -> bytes::Bytes {
    let item = MetadataDictionaryItem { key: Key::FlagsExtended, r#type: Type::Long, legacy_type: 7, value: Value::FlagsExtended(MetadataFlags2(REST | set)) };
    sapling_net::encode(&SetEntityData { runtime_entity_id: id, metadata: vec![item], properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 })
}

/// Before its move, with its prey at `prey`: whether it stands, readies, leaps or walks.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body, prey: Option<[f32; 3]>) -> bool {
    let now = world.resource::<Tick>().0;
    let Some(stage) = world.get::<Breeze>(entity).map(|breeze| breeze.stage) else { return false };
    let cell = body.at.map(|c| c.floor() as i32);
    let tell = |world: &mut World, set: i64| entities::tell_viewers(world, entity, layer, cell, vec![flags(id, set)]);
    let Some(feet) = prey else {
        if matches!(stage, Stage::Facing(_)) {
            tell(world, 0);
        }
        let airborne = matches!(stage, Stage::Leaping(_) | Stage::Readying(..));
        if !airborne {
            set(world, entity, Stage::Idle);
        }
        return airborne;
    };
    let far = (feet[0] - body.at[0]).hypot(feet[2] - body.at[2]);
    let next = match stage {
        Stage::Idle => {
            tell(world, FACING);
            Stage::Facing(now)
        }
        Stage::Facing(since) if now >= since + FACES => {
            charges::fire(world, layer, entity, kind, body, feet);
            Stage::After(now)
        }
        Stage::After(shot) if now >= shot + AFTER => {
            let roll = world.get_resource_or_insert_with(Dice::default).fraction();
            let speed = if body.on_ground { leap::pick(world, entity, layer, body, feet) } else { None };
            match (speed, far) {
                (Some(speed), _) => {
                    tell(world, LEAPING);
                    Stage::Readying(now, speed)
                }
                (None, far) if far < BACKS_TO[0] => {
                    tell(world, 0);
                    let (angle, out) = (roll * std::f32::consts::TAU, BACKS_TO[0] + roll * (BACKS_TO[1] - BACKS_TO[0]));
                    Stage::Walking([feet[0] + out * angle.cos(), feet[2] + out * angle.sin()], now)
                }
                _ => {
                    tell(world, 0);
                    Stage::Waiting(now + WAITS)
                }
            }
        }
        Stage::Readying(since, speed) if now >= since + READIES => {
            if let Some(mut moving) = world.get_mut::<Body>(entity) {
                moving.launch(speed, LIFT);
            }
            late_sounds::play_later(world, 1, layer, cell, hunting::sound("jump", id, kind, body, 0.0));
            Stage::Leaping(now)
        }
        Stage::Leaping(since) if now > since + 1 && body.on_ground => {
            tell(world, 0);
            Stage::Landed
        }
        Stage::Landed => {
            tell(world, FACING);
            Stage::Facing(now)
        }
        Stage::Waiting(until) if now >= until => {
            tell(world, FACING);
            Stage::Facing(now)
        }
        Stage::Walking(to, since) => {
            let done = (to[0] - body.at[0]).hypot(to[1] - body.at[2]) < ARRIVED || now >= since + WALKS_FOR;
            if !done {
                legs::walk(world, entity, layer, body, to, SPEED, 0.0);
                return true;
            }
            tell(world, FACING);
            Stage::Facing(now)
        }
        stage => stage,
    };
    set(world, entity, next);
    if !matches!(next, Stage::Leaping(_)) {
        legs::rest(world, entity);
    }
    true
}

fn set(world: &mut World, entity: Entity, stage: Stage) {
    if let Some(mut breeze) = world.get_mut::<Breeze>(entity) {
        breeze.stage = stage;
    }
}
