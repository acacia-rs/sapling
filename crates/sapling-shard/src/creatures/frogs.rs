//! A frog on land, as BDS's sat and jumped in the `amphibians` drill
//! (research/captures/bds-amphibians.txt, research/captures/bds-gameplay-findings.md): it stays
//! put and, 5 to 7 seconds and a 2-second run-up after it last landed (`jump_to_block`'s
//! cooldown_range), leaps at 70 degrees to a block within 8 across and 4 up or down, a lily
//! pad or big dripleaf half the time there is one. The leap is told every tick with no drag,
//! gravity 0.08; on landing it keeps a tenth of its speed across.
// TODO: the run-up is fitted (BDS's waits from a landing were 7.5 to 9.2 s); the way to the
// block is not checked for walls; a frog does not croak, eat or lay spawn.

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
const LANDING_KEEPS: f32 = 0.1;
const PREFERRED: [&str; 2] = ["minecraft:waterlily", "minecraft:big_dripleaf"];
const PREFERRED_CHANCE: f32 = 0.5;
const FORBIDDEN: &str = "minecraft:water";

#[derive(Clone, Copy)]
enum Stage {
    Sitting(u64),
    Leaping([f32; 3], u64),
}

#[derive(Component)]
pub(super) struct Frog {
    stage: Stage,
}

pub(super) fn arm(world: &mut World, entity: Entity, kind: &str) {
    if kind == KIND {
        let until = wait(world);
        world.entity_mut(entity).insert(Frog { stage: Stage::Sitting(until) });
    }
}

fn wait(world: &mut World) -> u64 {
    let now = world.resource::<Tick>().0;
    now + WAITS[0] + u64::from(world.get_resource_or_insert_with(Dice::default).roll((WAITS[1] - WAITS[0]) as u32 + 1))
}

/// Before its move: a frog in the air keeps its leap; on land it sits, and leaps when it is
/// time. False in water, where it swims (swimming.rs).
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity, id: u64) -> bool {
    let (Some(stage), Some(body)) = (world.get::<Frog>(entity).map(|frog| frog.stage), world.get::<Body>(entity).copied()) else { return false };
    let now = world.resource::<Tick>().0;
    let cell = body.at.map(|c| c.floor() as i32);
    let next = match stage {
        Stage::Leaping([x, _, z], since) if now > since + 1 && body.on_ground => {
            fling(world, entity,[x * LANDING_KEEPS, -GRAVITY, z * LANDING_KEEPS]);
            entities::tell_viewers(world, entity, layer, cell, vec![breezes::flags(id, 0)]);
            Stage::Sitting(wait(world))
        }
        Stage::Leaping([x, y, z], since) => {
            fling(world, entity,[x, y - GRAVITY * (now - since) as f32, z]);
            stage
        }
        _ if body.liquid() == Some(Liquid::Water) => return false,
        Stage::Sitting(until) if now >= until && body.on_ground => match target(world, layer, &body) {
            Some(speed) => {
                fling(world, entity,speed);
                entities::tell_viewers(world, entity, layer, cell, vec![breezes::flags(id, breezes::LEAPING)]);
                late_sounds::play_later(world, 1, layer, cell, hunting::sound("jump_to_block", id, KIND, &body, 0.0));
                Stage::Leaping(speed, now)
            }
            None => Stage::Sitting(wait(world)),
        },
        Stage::Sitting(_) => stage,
    };
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
    (far >= NEAREST).then(|| {
        let (cos, sin) = (ANGLE.to_radians().cos(), ANGLE.to_radians().sin());
        [dx / far * speed * cos, speed * sin, dz / far * speed * cos]
    })
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
