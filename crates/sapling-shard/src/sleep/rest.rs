//! When a bed lets its clicker sleep: by night or in a thunderstorm, and with no monster near
//! (research/captures/bds-rest.txt).

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Weather};

use crate::body::Body;
use crate::components::{InLayer, Layer};
use crate::entities::Kind;

/// The ticks of a clear day in which one may sleep: the sky is dark enough (BDS `skyDarken` 4).
const NIGHT: std::ops::RangeInclusive<i64> = 12_541..=23_459;
/// How far from the bed's head a monster keeps a sleeper awake, across and up or down: a zombie 6
/// blocks off did, one 9 off did not.
const MONSTER_REACH: [f32; 3] = [8.0, 5.0, 8.0];

/// The same in rain (docs/research/beds-and-containers.md, "Night window").
const RAINY_NIGHT: std::ops::RangeInclusive<i64> = 12_009..=23_991;

pub(super) fn is_night(world: &World, layer: Entity) -> bool {
    let night = if crate::weather::of(world, layer) == Weather::Rain { RAINY_NIGHT } else { NIGHT };
    world.get::<Layer>(layer).is_some_and(|layer| night.contains(&layer.world.level().time.rem_euclid(super::DAY)))
}

/// Whether the sky of `layer` lets anyone sleep: night, or a storm. A storm's sleep by day
/// skips nothing: BDS left a sleeper 8 s in bed at noon.
pub(super) fn is_resting_time(world: &World, layer: Entity) -> bool {
    is_night(world, layer) || crate::weather::of(world, layer) == Weather::Thunder
}

/// Whether a monster of `layer` is within [`MONSTER_REACH`] of the bed whose head is at `head`:
/// any of the `monster` family, a zombified piglin too.
pub(super) fn monster_near(world: &mut World, layer: Entity, head: BlockPos) -> bool {
    let centre = [head[0] as f32 + 0.5, head[1] as f32, head[2] as f32 + 0.5];
    let min: [f32; 3] = std::array::from_fn(|axis| centre[axis] - MONSTER_REACH[axis]);
    let max: [f32; 3] = std::array::from_fn(|axis| centre[axis] + MONSTER_REACH[axis]);
    let mut mobs = world.query::<(&InLayer, &Kind, &Body)>();
    mobs.iter(world).any(|(in_layer, kind, body)| {
        let [half, height] = [body.size[0] / 2.0, body.size[1]];
        let low = [body.at[0] - half, body.at[1], body.at[2] - half];
        let high = [body.at[0] + half, body.at[1] + height, body.at[2] + half];
        in_layer.0 == layer && (0..3).all(|axis| low[axis] < max[axis] && high[axis] > min[axis]) && sapling_data::has_family(&kind.0, "monster")
    })
}
