//! A frog on land, as BDS's sat and jumped in the `amphibians` drill
//! (research/captures/bds-amphibians.txt, research/captures/bds-gameplay-findings.md): it stays
//! put and, 5 to 7 seconds and a 2-second run-up after it last landed (`jump_to_block`'s
//! cooldown_range), leaps at 70 degrees to a block within 8 across and 4 up or down, a lily
//! pad or big dripleaf half the time there is one. The leap is told every tick with no drag,
//! gravity 0.08; on landing it keeps a tenth of its speed across.
// The run-up is fitted: BDS's waits from a landing were 7.3 to 9.2 s, sapling's 7.1 to 9.0
// (research/captures/bds-n3frogs.txt, sapling-n3frogs.txt).
//! It eats small slimes and magma cubes (eating.rs) and lays frogspawn (husbandry/frog_spawn.rs).

mod eating;

use bevy_ecs::prelude::*;
use sapling_world::World as Level;

use crate::blocks::Dice;
use crate::body::{Body, Liquid};
use crate::components::{Layer, Tick};
use crate::{entities, late_sounds};
use super::{breezes, harm, hunting};

const KIND: &str = "minecraft:frog";
/// Ticks from a landing to the next leap.
const WAITS: [u64; 2] = [140, 180];
const ACROSS: i32 = 8;
const UP: i32 = 4;
const NEAREST: f32 = 1.0;
const FASTEST: f32 = 1.0;
const ANGLE: f32 = 70.0;
const GRAVITY: f32 = 0.08;
/// The ticks of a leap looked along for blocks in its way.
const LEAP_TICKS: u32 = 60;
const LANDING_KEEPS: f32 = 0.1;
const PREFERRED: [&str; 2] = ["minecraft:waterlily", "minecraft:big_dripleaf"];
const PREFERRED_CHANCE: f32 = 0.5;
const FORBIDDEN: &str = "minecraft:water";
/// Ticks between croaks (`interval` 10 to 20 s), how long one lasts, and its `FlagsExtended` bit.
const CROAK_GAPS: [u64; 2] = [200, 400];
const CROAKS: u64 = 90;
const CROAKING: i64 = 1 << 37;

#[derive(Clone, Copy)]
enum Stage {
    Sitting(u64),
    Leaping([f32; 3], u64),
}

#[derive(Component)]
pub(super) struct Frog {
    stage: Stage,
    /// The tick its next croak may start in, and the tick the one under way ends in.
    croaks: u64,
    croaking: Option<u64>,
}

pub(super) fn arm(world: &mut World, entity: Entity, kind: &str) {
    if kind == KIND {
        let (until, croaks) = (wait(world, WAITS), wait(world, CROAK_GAPS));
        world.entity_mut(entity).insert(Frog { stage: Stage::Sitting(until), croaks, croaking: None });
    }
}

fn wait(world: &mut World, [least, most]: [u64; 2]) -> u64 {
    let now = world.resource::<Tick>().0;
    now + least + u64::from(world.get_resource_or_insert_with(Dice::default).roll((most - least) as u32 + 1))
}

/// Its `FlagsExtended` with `set`, and its croak's bit while it croaks.
fn flags(world: &World, entity: Entity, id: u64, set: i64) -> bytes::Bytes {
    let croaking = world.get::<Frog>(entity).is_some_and(|frog| frog.croaking.is_some());
    breezes::flags(id, set | if croaking { CROAKING } else { 0 })
}

/// A tick of a frog on land: it starts a croak when one is due and ends it [`CROAKS`] after
/// (`behavior.croak`; BDS's showed `FlagsExtended` bit 37 for 90 ticks, 10.8 to 14.1 s apart,
/// with no sound, research/captures/bds-g14frog.txt).
fn croak(world: &mut World, entity: Entity, layer: Entity, cell: [i32; 3], id: u64, leaping: bool) {
    let now = world.resource::<Tick>().0;
    let Some(frog) = world.get::<Frog>(entity) else { return };
    let (croaks, croaking) = (frog.croaks, frog.croaking);
    let next = match croaking {
        Some(until) if now >= until => Some((None, wait(world, CROAK_GAPS))),
        None if now >= croaks => Some((Some(now + CROAKS), croaks)),
        _ => None,
    };
    let Some((croaking, croaks)) = next else { return };
    if let Some(mut frog) = world.get_mut::<Frog>(entity) {
        (frog.croaking, frog.croaks) = (croaking, croaks);
    }
    let packet = flags(world, entity, id, if leaping { breezes::LEAPING } else { 0 });
    entities::tell_viewers(world, entity, layer, cell, vec![packet]);
}

/// Before its move: a frog in the air keeps its leap; on land it sits, and leaps when it is
/// time. False in water, where it swims (swimming.rs).
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity, id: u64) -> bool {
    let (Some(stage), Some(body)) = (world.get::<Frog>(entity).map(|frog| frog.stage), world.get::<Body>(entity).copied()) else { return false };
    let now = world.resource::<Tick>().0;
    let cell = body.at.map(|c| c.floor() as i32);
    if matches!(stage, Stage::Sitting(_)) && body.liquid() != Some(Liquid::Water) && eating::steer(world, entity, layer, id, &body) {
        croak(world, entity, layer, cell, id, false);
        return true;
    }
    let next = match stage {
        Stage::Leaping([x, _, z], since) if now > since + 1 && body.on_ground => {
            fling(world, entity,[x * LANDING_KEEPS, -GRAVITY, z * LANDING_KEEPS]);
            let packet = flags(world, entity, id, 0);
            entities::tell_viewers(world, entity, layer, cell, vec![packet]);
            Stage::Sitting(wait(world, WAITS))
        }
        Stage::Leaping([x, y, z], since) => {
            fling(world, entity,[x, y - GRAVITY * (now - since) as f32, z]);
            stage
        }
        _ if body.liquid() == Some(Liquid::Water) => return false,
        Stage::Sitting(until) if now >= until && body.on_ground => match target(world, layer, &body) {
            Some(speed) => {
                fling(world, entity,speed);
                let packet = flags(world, entity, id, breezes::LEAPING);
                entities::tell_viewers(world, entity, layer, cell, vec![packet]);
                late_sounds::play_later(world, 1, layer, cell, hunting::sound("jump_to_block", id, KIND, &body, 0.0));
                Stage::Leaping(speed, now)
            }
            None => Stage::Sitting(wait(world, WAITS)),
        },
        Stage::Sitting(_) => stage,
    };
    croak(world, entity, layer, cell, id, matches!(next, Stage::Leaping(..)));
    if let Some(mut frog) = world.get_mut::<Frog>(entity) {
        frog.stage = next;
    }
    super::legs::rest(world, entity);
    true
}

fn fling(world: &mut World, entity: Entity, speed: [f32; 3]) {
    if let Some(mut body) = world.get_mut::<Body>(entity) {
        body.fling(speed);
    }
}

/// The leap to a block it may land on near `body`, picked as `jump_to_block` picks; none where
/// there is none in reach.
fn target(world: &mut World, layer: Entity, body: &Body) -> Option<[f32; 3]> {
    let level = &world.get::<Layer>(layer)?.world;
    let [fx, fy, fz] = body.at.map(|c| c.floor() as i32);
    let mut found = Vec::new();
    for dx in -ACROSS..=ACROSS {
        for dz in -ACROSS..=ACROSS {
            for dy in -UP..=UP {
                let support = [fx + dx, fy + dy - 1, fz + dz];
                if let Some(speed) = landing(level, body, support) {
                    found.push((speed, preferred(level, support)));
                }
            }
        }
    }
    let mut dice = world.get_resource_or_insert_with(Dice::default);
    let favoured: Vec<[f32; 3]> = found.iter().filter(|(_, preferred)| *preferred).map(|(speed, _)| *speed).collect();
    let pool: Vec<[f32; 3]> = if !favoured.is_empty() && dice.fraction() < PREFERRED_CHANCE { favoured } else { found.into_iter().map(|(speed, _)| speed).collect() };
    (!pool.is_empty()).then(|| pool[dice.roll(pool.len() as u32) as usize])
}

fn preferred(level: &Level, [x, y, z]: [i32; 3]) -> bool {
    harm::holds(level, [x, y + 1, z], &PREFERRED) || harm::holds(level, [x, y, z], &PREFERRED)
}

/// The leap from `body` onto the block `support`, where it may stand on it.
fn landing(level: &Level, body: &Body, [x, y, z]: [i32; 3]) -> Option<[f32; 3]> {
    let is_air = |y: i32| level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_air());
    let stands = harm::is_full_block(level, [x, y, z]) || preferred(level, [x, y, z]);
    if !stands || !is_air(y + 1) || !is_air(y + 2) || harm::holds(level, [x, y, z], &[FORBIDDEN]) || harm::holds(level, [x, y + 1, z], &[FORBIDDEN]) {
        return None;
    }
    let (dx, dz) = (x as f32 + 0.5 - body.at[0], z as f32 + 0.5 - body.at[2]);
    let far = dx.hypot(dz);
    let speed = leap(far, y as f32 - body.at[1])?;
    let (cos, sin) = (ANGLE.to_radians().cos(), ANGLE.to_radians().sin());
    let speed = [dx / far * speed * cos, speed * sin, dz / far * speed * cos];
    (far >= NEAREST && clear_way(level, body, speed, y as f32 + 1.0)).then_some(speed)
}

/// Whether the leap at `speed` from `body` meets no block before it comes down to `top`: BDS's frog
/// in a glass pen 6 high leapt only within it in 16 s (research/captures/bds-g14frog.txt).
fn clear_way(level: &Level, body: &Body, speed: [f32; 3], top: f32) -> bool {
    let mut at = body.at;
    for tick in 0..LEAP_TICKS {
        let rise = speed[1] - GRAVITY * tick as f32;
        let landed = rise < 0.0 && at[1] + rise <= top;
        at = [at[0] + speed[0], if landed { top } else { at[1] + rise }, at[2] + speed[2]];
        if !Body::new(at, [0.0; 3], body.size).is_free(level) {
            return false;
        }
        if landed {
            return true;
        }
    }
    true
}

/// The speed of a leap at [`ANGLE`] that comes down `rise` higher `far` off: v² = g·far² /
/// (far·sin 2θ − 2·rise·cos²θ), BDS's leaps fitted to 0.3% (a leap to the block 1 and 4 off,
/// level with the one it sat on, went at 0.689). None past [`FASTEST`].
fn leap(far: f32, rise: f32) -> Option<f32> {
    let angle = ANGLE.to_radians();
    let under = far * (2.0 * angle).sin() - 2.0 * rise * angle.cos().powi(2);
    let speed = (GRAVITY * far * far / under).sqrt();
    (under > 0.0 && speed <= FASTEST).then_some(speed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_leap_to_a_block_one_and_four_off_goes_as_bds_s_did() {
        let speed = leap(17f32.sqrt(), -1.0).unwrap();
        assert!((speed - 0.689).abs() < 0.003, "{speed}");
        assert!(leap(30.0, 0.0).is_none());
    }
}
