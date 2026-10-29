//! Turtle eggs (`TurtleEggBlock`, docs/research/mob-life.md "Turtle eggs"): placed on sand they
//! sparkle, and the first tick comes 1000 ticks later. A tick at dawn, or one in 500 at any other
//! time, cracks an egg on sand once more, or hatches one cracked twice into a baby turtle per egg
//! whose home is the egg's cell; then the next tick is queued 750 to 1499 ticks on. A living thing
//! but a turtle or bat breaks an egg treading on it, one in 100 a tick and one in 3 landing on it,
//! a mob only with `mobGriefing` on, a player sneaking or not.
// Laying and stomping out: nests.rs.

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::{Level, again, crack, cracks, egg_sound, hatch_burst, name_at, roll, sparkle, value_of};
use crate::blocks::{level, send};

pub(super) const EGG: &str = "minecraft:turtle_egg";
const TURTLE: &str = "minecraft:turtle";
pub(super) const COUNTS: [&str; 4] = ["one_egg", "two_egg", "three_egg", "four_egg"];
pub(super) const SANDS: [&str; 3] = ["minecraft:sand", "minecraft:red_sand", "minecraft:suspicious_sand"];
const FIRST_TICK: u64 = 1000;
/// Ticks to the next tick: the least, and how many more it may be.
const NEXT_TICK: (u64, u32) = (750, 750);
/// The sun's turn (`light::sun_turn`) between which every tick cracks, and the odds at other times.
const DAWN: (f64, f64) = (0.65, 0.69);
const ODDS: u32 = 500;
/// Every turtle of an egg block comes out at the cell's corner, this far along x (BDS spawns at the
/// corner and announces it 0.12 west: research/captures/bds-eggs.txt).
const HATCH_X: f32 = -0.12;
/// The odds of breaking an egg stood on for a tick, and landed on.
const STEP_ODDS: u32 = 100;
const FALL_ODDS: u32 = 3;
/// Feet this far over a cell stand on it (the Java game's `getOnPos`).
const ON_UNDER: f32 = 0.2;
/// Kinds that never break eggs; a zombie breaks none landing (its stomping is a goal of its own).
const SPARE: [&str; 2] = [TURTLE, "minecraft:bat"];
const ZOMBIE: &str = "minecraft:zombie";
const PLAYER: &str = "minecraft:player";
const GRIEFING: &str = "mobGriefing";

fn on_sand(level: &Level, [x, y, z]: BlockPos) -> bool {
    name_at(level, [x, y - 1, z]).is_some_and(|under| SANDS.contains(&under))
}

fn eggs(level: &Level, pos: BlockPos) -> usize {
    value_of(level, pos, "turtle_egg_count").and_then(|count| COUNTS.iter().position(|eggs| *eggs == count)).unwrap_or(0) + 1
}

/// A turtle egg came at `pos`.
pub(super) fn placed(world: &mut World, layer: Entity, pos: BlockPos) {
    if let Some(sparkle) = level(world, layer).filter(|level| on_sand(level, pos)).map(|level| sparkle(level, pos)) {
        send::to_viewers(world, layer, pos, &[sparkle]);
    }
    again(world, layer, pos, FIRST_TICK);
}

/// The queued tick of the turtle egg at `pos` (`TurtleEggBlock`'s tick, 14257e040).
pub(super) fn ticked(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(level) = level(world, layer) else { return };
    let sun = f64::from(crate::light::sun_turn(level.level().time));
    let sanded = on_sand(level, pos);
    let cracked = cracks(level, pos);
    let due = (sun > DAWN.0 && sun < DAWN.1) || roll(world, ODDS) == 0;
    if due && sanded {
        if cracked >= 2 {
            return hatch(world, layer, pos);
        }
        crack(world, layer, pos, "block.turtle_egg.crack", false);
    }
    let delay = NEXT_TICK.0 + u64::from(roll(world, NEXT_TICK.1));
    again(world, layer, pos, delay);
}

/// The egg at `pos` hatches: it goes, and a baby turtle comes out of each egg, at home there.
fn hatch(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(level) = level(world, layer) else { return };
    let (id, air, count) = (level.block(pos[0], pos[1], pos[2]), level.registry().air_id(), eggs(level, pos));
    let burst = hatch_burst(level, pos, id);
    send::to_viewers(world, layer, pos, &vec![burst; count]);
    if !send::set(world, layer, pos, air) {
        return;
    }
    let [x, y, z] = pos.map(|c| c as f32);
    for _ in 0..count {
        if let Some(turtle) = crate::creatures::hatch(world, layer, TURTLE, [x + HATCH_X, y, z]) {
            crate::creatures::make_home(world, turtle, pos);
        }
    }
    send::to_viewers(world, layer, pos, &[egg_sound("block.turtle_egg.hatch", pos)]);
}

/// One egg of those at `pos` breaks, the last taking the block with it: particles, the block,
/// then the sound, all at the cell's corner (research/captures/bds-eggtrample.txt).
pub(super) fn break_one(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(level) = level(world, layer) else { return };
    let (id, count) = (level.block(pos[0], pos[1], pos[2]), eggs(level, pos));
    let fewer = COUNTS.get(count.wrapping_sub(2)).map_or(Some(level.registry().air_id()), |fewer| crate::blocks::with_property(level.registry(), id, "turtle_egg_count", fewer));
    let Some(fewer) = fewer else { return };
    send::to_viewers(world, layer, pos, &[hatch_burst(level, pos, id)]);
    if send::set(world, layer, pos, fewer) {
        send::to_viewers(world, layer, pos, &[egg_sound("block.turtle_egg.break", pos)]);
    }
}

/// A tick of a living thing of `kind` with its feet at `feet`: on the ground over a turtle egg it
/// may break one, more likely when it `landed` this tick. Sneaking spares none on BDS.
pub(crate) fn trodden(world: &mut World, layer: Entity, kind: &str, feet: [f32; 3], on_ground: bool, landed: bool) {
    if !on_ground || SPARE.contains(&kind) {
        return;
    }
    // TODO: sapling's mobs sink into an egg's cell where BDS's stand on its top (y -25.55 in
    // research/captures/bds-eggtrample.txt), so the cell at the feet counts too.
    let cell = |y: f32| [feet[0].floor() as i32, y.floor() as i32, feet[2].floor() as i32];
    let pos = Some(cell(feet[1])).filter(|inside| is_egg_at(world, layer, *inside)).unwrap_or_else(|| cell(feet[1] - ON_UNDER));
    if !is_egg_at(world, layer, pos) || kind != PLAYER && !level(world, layer).is_some_and(|level| level.level().rules.on(GRIEFING)) {
        return;
    }
    if landed && kind != ZOMBIE && roll(world, FALL_ODDS) == 0 {
        break_one(world, layer, pos);
    }
    if is_egg_at(world, layer, pos) && roll(world, STEP_ODDS) == 0 {
        break_one(world, layer, pos);
    }
}

pub(super) fn is_egg_at(world: &World, layer: Entity, pos: BlockPos) -> bool {
    level(world, layer).is_some_and(|level| name_at(level, pos) == Some(EGG))
}
