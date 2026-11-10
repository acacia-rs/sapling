//! A squid pressed against a block, as Linux BDS's `SquidMoveAwayFromGroundGoal` (vtable 0xeefca20)
//! moves it: the goal runs while the squid has a `CollisionFlagComponent` (canUse 0x5933820 and
//! canContinueToUse 0x59338d0 test only that) and outranks the idle goal. Its start (0x5933980) takes
//! the out-of-water goal's random way (0x7211330), unit length, and casts it 5 blocks (0x7211530):
//! where that meets a block it casts the way turned 30 degrees either side, flat, and keeps the side
//! that runs clear, the farther-reaching where both meet one, a coin's pick where neither does. Its
//! new swim way is that times 0.0025, a tenth of a swim: BDS's squids in a pool 5 across crept along
//! its walls at 0.0009 a tick for seconds (research/captures/bds-ab1squid.txt). The idle goal runs
//! alongside it, never stopped: its pick every 201 ticks goes on while the squid is pressed and is not
//! hurried by the release (bds-ah1swim gdb: a creep off the wall ran on past the goal's one tick; one
//! pressed along a wall crept 4.5 s until the idle pick took it off). The flag is set by a move a
//! block cut short on any axis (body/moves.rs `collided`), not by a block merely touching.

use bevy_ecs::prelude::*;
use sapling_world::World as Level;

use super::ashore::{self, Ashore};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::Layer;

const CAST: f32 = 5.0;
const STEP: f32 = 0.05;
const SCALE: f32 = 0.0025;
/// cos and sin of the 30-degree turn.
const TURN: [f32; 2] = [0.866_025_4, 0.5];

/// How far along `way` (unit length) from `from` the first solid block is, within [`CAST`].
fn cast(level: &Level, from: [f32; 3], way: [f32; 3]) -> Option<f32> {
    let solid = |[x, y, z]: [i32; 3]| level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_solid());
    (1..=(CAST / STEP) as usize).map(|step| step as f32 * STEP).find(|far| solid(std::array::from_fn(|axis| (from[axis] + way[axis] * far).floor() as i32)))
}

/// The way the goal's start gives a squid at `body` in `layer`.
pub(super) fn way(world: &mut World, layer: Entity, body: &Body) -> [f32; 3] {
    let picked = ashore::way(&mut world.get_resource_or_insert_with(Dice::default), Ashore::Stranded);
    let long = picked.iter().map(|c| c * c).sum::<f32>().sqrt();
    let unit = picked.map(|c| c / long);
    let [x, _, z] = unit;
    let [cos, sin] = TURN;
    let left = [cos * x + sin * z, 0.0, cos * z - sin * x];
    let right = [cos * x - sin * z, 0.0, cos * z + sin * x];
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return [0.0; 3] };
    let kept = if cast(level, body.at, unit).is_none() {
        Some(unit)
    } else {
        match (cast(level, body.at, left), cast(level, body.at, right)) {
            (Some(_), None) => Some(right),
            (None, Some(_)) => Some(left),
            (Some(a), Some(b)) => Some(if b <= a { left } else { right }),
            (None, None) => None,
        }
    };
    let kept = kept.unwrap_or_else(|| if world.get_resource_or_insert_with(Dice::default).roll(2) == 0 { right } else { left });
    kept.map(|c| c * SCALE)
}
