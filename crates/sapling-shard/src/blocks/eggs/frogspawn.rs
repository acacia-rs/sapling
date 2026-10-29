//! Frogspawn (`FrogSpawnBlock`, docs/research/mob-life.md "Frogspawn"): placed, its tick comes 3600
//! to 11999 ticks later and hatches it into 2 to 6 tadpoles in the water under it, each at a random
//! spot of the cell; frogspawn with no water under it breaks instead, leaving nothing.

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::{Level, again, egg_sound, name_at, roll};
use crate::blocks::{Dice, level, send};

pub(super) const SPAWN: &str = "minecraft:frog_spawn";
const TADPOLE: &str = "minecraft:tadpole";
const WATER: [&str; 2] = ["minecraft:water", "minecraft:flowing_water"];
/// Ticks to the hatching: the least, and how many more it may be.
const HATCHES_IN: (u64, u32) = (3600, 8400);
/// Tadpoles hatched: the least, and how many more there may be.
const TADPOLES: (u32, u32) = (2, 5);
/// Tadpoles come this far down from the spawn's cell, into the water.
const DOWN: f32 = 0.5;

fn afloat(level: &Level, [x, y, z]: BlockPos) -> bool {
    name_at(level, [x, y - 1, z]).is_some_and(|under| WATER.contains(&under))
}

/// Frogspawn came at `pos`: on water it is heard laid (research/captures/bds-eggs.txt).
pub(super) fn placed(world: &mut World, layer: Entity, pos: BlockPos) {
    if level(world, layer).is_some_and(|level| afloat(level, pos)) {
        send::to_viewers(world, layer, pos, &[egg_sound("lay_spawn", pos)]);
    }
    let delay = HATCHES_IN.0 + u64::from(roll(world, HATCHES_IN.1));
    again(world, layer, pos, delay);
}

/// The queued tick of the frogspawn at `pos` (`FrogSpawnBlock`'s tick, 147160310).
pub(super) fn ticked(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(level) = level(world, layer) else { return };
    let [x, y, z] = pos;
    let (id, air) = (level.block(x, y, z), level.registry().air_id());
    let (heard, burst) = (egg_sound("block.frog_spawn.hatch", pos), send::destroy_particles(level.registry(), pos, id));
    let afloat = afloat(level, pos);
    if !send::set(world, layer, pos, air) {
        return;
    }
    if !afloat {
        return send::to_viewers(world, layer, pos, &[burst]);
    }
    send::to_viewers(world, layer, pos, &[heard]);
    let count = TADPOLES.0 + roll(world, TADPOLES.1);
    for _ in 0..count {
        send::to_viewers(world, layer, pos, std::slice::from_ref(&burst));
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        let (across, along) = (dice.fraction(), dice.fraction());
        crate::creatures::hatch(world, layer, TADPOLE, [x as f32 + across, y as f32 - DOWN, z as f32 + along]);
    }
}
