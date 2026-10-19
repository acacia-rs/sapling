//! A breeze, as BDS's fought (research/captures/bds-breeze-findings.md): it faces its prey for 16
//! ticks and shoots a wind charge (`charges`); 4 ticks on it leaps at prey 4 to 20 off, after
//! readying for 10, or walks out to 4 to 8 off prey nearer, then faces it again.
// TODO: a leap lands on its prey, where BDS's landed 1 to 4 off by a rule not found; it leaps
// over nothing in its way, its own bursts do not push it, and it does not turn as it faces.

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
/// It leaps at prey this far off, at one of these angles, at most this fast; the first tick of
/// a leap goes this much higher, and each after falls this much more.
const LEAPS: [f32; 2] = [4.0, 20.0];
const ANGLES: [f32; 5] = [40.0, 55.0, 60.0, 75.0, 80.0];
const FASTEST: f32 = 1.4;
const LIFT: f32 = 0.2655;
const GRAVITY: f32 = 0.08;
/// Nearer prey it walks from, this fast, to this far off it; a walk ends this near its end.
const BACKS_TO: [f32; 2] = [4.0, 8.0];
const SPEED: f32 = 0.48;
const ARRIVED: f32 = 0.6;
/// Its `FlagsExtended` at rest, facing its prey and leaping: bits 24 and 39 as BDS sent them,
/// one above acacia-proto's names for them.
const REST: i64 = i64::MIN | 1 << 25;
const FACING: i64 = 1 << 24;
const LEAPING: i64 = 1 << 39;

#[derive(Clone, Copy, Default)]
enum Stage {
    #[default]
    Idle,
    Facing(u64),
    After(u64),
    Readying(u64),
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

fn flags(id: u64, set: i64) -> bytes::Bytes {
    let item = MetadataDictionaryItem { key: Key::FlagsExtended, r#type: Type::Long, legacy_type: 7, value: Value::FlagsExtended(MetadataFlags2(REST | set)) };
    sapling_net::encode(&SetEntityData { runtime_entity_id: id, metadata: vec![item], properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 })
}

/// The leap at `angle` degrees that lands `far` off and `rise` higher, by this game's steps; the
/// fastest where none reaches.
fn leap(angle: f32, far: f32, rise: f32) -> [f32; 2] {
    let (cos, sin) = (angle.to_radians().cos(), angle.to_radians().sin());
    let lands = |speed: f32| {
        let (across, up) = (speed * cos, speed * sin);
        let mut height = up + LIFT;
        let mut ticks = 1.0;
        while height > rise || ticks < 2.0 {
            height += up - GRAVITY * ticks;
            ticks += 1.0;
            if ticks > 200.0 {
                break;
            }
        }
        across * ticks
    };
    let (mut low, mut high) = (0.0, FASTEST);
    for _ in 0..30 {
        let middle = (low + high) / 2.0;
        if lands(middle) < far { low = middle } else { high = middle }
    }
    [high * cos, high * sin]
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
        let airborne = matches!(stage, Stage::Leaping(_) | Stage::Readying(_));
        if !airborne {
            set(world, entity, Stage::Idle);
        }
        return airborne;
    };
    let (dx, dz) = (feet[0] - body.at[0], feet[2] - body.at[2]);
    let far = dx.hypot(dz);
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
            match far {
                far if (LEAPS[0]..=LEAPS[1]).contains(&far) && body.on_ground => {
                    tell(world, LEAPING);
                    Stage::Readying(now)
                }
                far if far < BACKS_TO[0] => {
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
        Stage::Readying(since) if now >= since + READIES => {
            let pick = world.get_resource_or_insert_with(Dice::default).roll(ANGLES.len() as u32) as usize;
            let [across, up] = leap(ANGLES[pick], far, feet[1] - body.at[1]);
            let way = if far > 1e-4 { [dx / far, dz / far] } else { [0.0; 2] };
            if let Some(mut moving) = world.get_mut::<Body>(entity) {
                moving.launch([way[0] * across, up, way[1] * across], LIFT);
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
