//! A goat's ram (docs/research/mob-attacks.md, "The goat"), as BDS's were seen to
//! (research/captures/bds-goat-findings.md): 30 to 300 seconds after it came and after each ram it
//! runs to a place 7 off its prey along an axis, stands, and charges along that axis into whoever
//! is in the way.
// TODO: a goat takes and drops prey between rams, as BDS's did, without showing it here; it rams
// players only, a baby as hard as a grown one, and a charge that meets nobody nor stops at a wall
// was not seen.

use acacia_proto::packets::{EntityEvent, EntityEventEventId, LevelSoundEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;

use super::{hunting, legs, paths};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{Layer, Position, Settings, Tick};
use crate::sync::Told;
use crate::{entities, late_sounds, vitals};

/// Ticks from one ram, or the goat's coming, to the next: its `cooldown_range` in seconds.
const RESTS: [u32; 2] = [600, 6000];
/// How far off a prey it rams; it runs to a place this far, or no nearer than the least.
const RAMS_FROM: [i32; 2] = [7, 4];
const SEES: f32 = 16.0;
/// Its movement times its `run_speed`.
const RUNS: f32 = 0.28;
/// It stands once this near its place, gives up a run after this many ticks, and charges this
/// long after it stood.
const ARRIVED: f32 = 0.2;
const GIVES_UP: u64 = 200;
const STANDS: u64 = 23;
/// Each tick's step of its charge, then the last again until it has gone [`CHARGES`].
const STEPS: [f32; 11] = [0.0282, 0.0583, 0.1131, 0.1888, 0.2859, 0.4043, 0.5458, 0.7090, 0.8950, 0.9965, 1.0521];
const CHARGES: f32 = 8.0;
/// A standing body's fall in a tick, which keeps the charge on the ground.
const SETTLING: f32 = -0.0784;
/// What its ram costs on normal difficulty, and its push: these times its step squared, across and up.
const HARM: f32 = 2.0;
const PUSH: [f32; 2] = [1.5, 0.18];
const PLAYER: [f32; 2] = [0.6, 1.8];
/// Its sounds come from its feet and from this much of its height up.
const MIDDLE: f32 = 0.5;

#[derive(Clone, Copy)]
enum Stage {
    Idle,
    Running { spot: [f32; 2], axis: [f32; 2], since: u64 },
    Standing { axis: [f32; 2], until: u64 },
    Charging { axis: [f32; 2], step: usize, gone: f32 },
}

#[derive(Component)]
pub(super) struct Ram {
    next: u64,
    stage: Stage,
}

fn rest(world: &mut World) -> u64 {
    world.get_resource_or_insert_with(Dice::default).roll(RESTS[1] - RESTS[0] + 1) as u64 + u64::from(RESTS[0])
}

/// Has the mob ram, after a first rest, if its kind does.
pub(super) fn arm(world: &mut World, entity: Entity, kind: &str) {
    if kind == "minecraft:goat" {
        let next = world.resource::<Tick>().0 + rest(world);
        world.entity_mut(entity).insert(Ram { next, stage: Stage::Idle });
    }
}

fn sound(name: &str, kind: &str, id: u64, at: [f32; 3]) -> bytes::Bytes {
    let [x, y, z] = at;
    sapling_net::encode(&LevelSoundEvent { sound_id: name.into(), position: Vec3f { x, y, z }, extra_data: -1, entity_type: kind.into(), is_baby_mob: false, is_global: false, entity_unique_id: id as i64, fire_at_position: None })
}

/// The axis from `prey` towards the goat at `at`, and how far along it the goat may stand: as far
/// as [`RAMS_FROM`] where every cell of the way is free, or none.
fn lane(world: &World, layer: Entity, at: [f32; 3], prey: [f32; 3]) -> Option<([f32; 2], i32)> {
    let level = &world.get::<Layer>(layer)?.world;
    let (dx, dz) = (at[0] - prey[0], at[2] - prey[2]);
    let axis = if dx.abs() > dz.abs() { [dx.signum(), 0.0] } else { [0.0, dz.signum()] };
    let cell = |along: i32| [(prey[0] + axis[0] * along as f32).floor() as i32, prey[1].floor() as i32, (prey[2] + axis[1] * along as f32).floor() as i32];
    let free = (1..=RAMS_FROM[0]).take_while(|along| paths::is_standable(level, cell(*along))).last()?;
    (free >= RAMS_FROM[1]).then_some((axis, free))
}

/// Before the goat's move: it starts, runs, stands or charges; false while it has no ram under way.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str) -> bool {
    let now = world.resource::<Tick>().0;
    let (Some(body), Some(ram)) = (world.get::<Body>(entity).copied(), world.get::<Ram>(entity).map(|ram| (ram.next, ram.stage))) else { return false };
    let stage = match ram {
        (next, Stage::Idle) if now >= next && body.on_ground && world.resource::<Settings>().0.difficulty != sapling_api::Difficulty::Peaceful => {
            let Some((axis, along)) = hunting::nearest(world, layer, &body, SEES).and_then(|(_, feet)| Some((lane(world, layer, body.at, feet)?, feet))).map(|((axis, along), feet)| (axis, [feet[0] + axis[0] * along as f32, feet[2] + axis[1] * along as f32])) else { return false };
            late_sounds::play_later(world, 1, layer, body.at.map(|c| c.floor() as i32), sound("mob.warning", kind, id, [body.at[0], body.at[1] + body.size[1] * MIDDLE, body.at[2]]));
            Stage::Running { spot: along, axis, since: now }
        }
        (_, Stage::Idle) => return false,
        (_, Stage::Running { spot, axis, since }) => {
            if (spot[0] - body.at[0]).hypot(spot[1] - body.at[2]) < ARRIVED {
                legs::rest(world, entity);
                late_sounds::play_later(world, 1, layer, body.at.map(|c| c.floor() as i32), sound("pre_ram", kind, id, body.at));
                Stage::Standing { axis, until: now + STANDS }
            } else if now >= since + GIVES_UP {
                end(world, entity);
                return false;
            } else {
                legs::walk(world, entity, layer, &body, spot, RUNS, 0.0);
                Stage::Running { spot, axis, since }
            }
        }
        (_, Stage::Standing { axis, until }) if now < until => {
            face(world, entity, axis);
            Stage::Standing { axis, until }
        }
        (_, Stage::Standing { axis, .. }) => Stage::Charging { axis: [-axis[0], -axis[1]], step: 0, gone: 0.0 },
        (_, Stage::Charging { gone, .. }) if gone >= CHARGES => {
            end(world, entity);
            return false;
        }
        (_, Stage::Charging { axis, step, gone }) => {
            let s = STEPS[step.min(STEPS.len() - 1)];
            face(world, entity, axis);
            if let Some(mut body) = world.get_mut::<Body>(entity) {
                body.throw([axis[0] * s, SETTLING, axis[1] * s]);
            }
            Stage::Charging { axis, step: step + 1, gone: gone + s }
        }
    };
    if let Some(mut ram) = world.get_mut::<Ram>(entity) {
        ram.stage = stage;
    }
    true
}

fn face(world: &mut World, entity: Entity, axis: [f32; 2]) {
    let yaw = (-axis[0]).atan2(axis[1]).to_degrees();
    if let Some(mut told) = world.get_mut::<Told>(entity) {
        told.face(yaw, yaw);
    }
    if let Some(mut legs) = world.get_mut::<legs::Legs>(entity) {
        legs.yaw = yaw;
    }
}

/// The ram is over: the next waits a rest.
fn end(world: &mut World, entity: Entity) {
    let next = world.resource::<Tick>().0 + rest(world);
    if let Some(mut ram) = world.get_mut::<Ram>(entity) {
        *ram = Ram { next, stage: Stage::Idle };
    }
}

/// After the goat's move: a charging goat whose box meets a player's rams it, heard a tick later.
pub(super) fn strike(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body) {
    let Some(Stage::Charging { axis, step, .. }) = world.get::<Ram>(entity).map(|ram| ram.stage) else { return };
    let across = (body.size[0] + PLAYER[0]) / 2.0;
    let meets = |feet: [f32; 3]| (feet[0] - body.at[0]).abs() < across && (feet[2] - body.at[2]).abs() < across && feet[1] < body.at[1] + body.size[1] && body.at[1] < feet[1] + PLAYER[1];
    let Some(player) = crate::components::in_world(world).into_iter().find(|player| world.get::<Position>(*player).is_some_and(|position| meets(position.feet))) else { return };
    let s = STEPS[(step.max(1) - 1).min(STEPS.len() - 1)];
    let push = [axis[0] * PUSH[0] * s * s, PUSH[1] * s * s, axis[1] * PUSH[0] * s * s];
    let harm = match world.resource::<Settings>().0.difficulty {
        sapling_api::Difficulty::Peaceful => 0.0,
        sapling_api::Difficulty::Easy => HARM.min(HARM / 2.0 + 1.0),
        sapling_api::Difficulty::Normal => HARM,
        sapling_api::Difficulty::Hard => HARM * 1.5,
    };
    let cell = body.at.map(|c| c.floor() as i32);
    let swing = EntityEvent { runtime_entity_id: id, event_id: EntityEventEventId::ArmSwing, data: 0, fire_at_position: None };
    entities::tell_viewers(world, entity, layer, cell, vec![sapling_net::encode(&swing)]);
    vitals::rammed(world, player, harm, kind, body.at, push);
    late_sounds::play_later(world, 1, layer, cell, sound("attack", kind, id, [body.at[0], body.at[1] + body.size[1] * MIDDLE, body.at[2]]));
    late_sounds::play_later(world, 1, layer, cell, sound("ram_impact", kind, id, body.at));
    end(world, entity);
}
