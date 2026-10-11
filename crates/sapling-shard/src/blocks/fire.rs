//! Fire: flint and steel or a fire charge lights it over a solid block or by one that burns, soul
//! fire over soul sand; it ages, spreads to what burns, burns that away and goes out
//! (docs/research/fire.md).
// TODO: the spread is the Java game's rule and odds, which BDS's burning planks and wool fit
// only loosely; rain puts none out; fire a world folder holds, and fire from lava or lightning,
// is not ticked.

use bevy_ecs::prelude::*;
use sapling_api::{Block, BlockPos, Difficulty, Face, Weather};

use super::{Dice, level, send, with_property};
use crate::components::{Settings, Tick};

const FIRE: &str = "minecraft:fire";
const SOUL_FIRE: &str = "minecraft:soul_fire";
const TNT: &str = "minecraft:tnt";
const AGE: &str = "age";
const OLDEST: u32 = 15;
/// Fire over these burns on.
const ETERNAL: [&str; 4] = ["minecraft:netherrack", "minecraft:magma", "minecraft:soul_sand", "minecraft:soul_soil"];
/// A fire's ticks are this far apart, and up to this much further.
const EVERY: [u32; 2] = [30, 10];
const RULE: &str = "doFireTick";
const FACES: [Face; 6] = [Face::Down, Face::Up, Face::North, Face::South, Face::West, Face::East];
/// One in this many is the chance a block beside, and over or under, a fire has to burn, times
/// what it burns by.
const BESIDE: u32 = 300;
const OVER: u32 = 250;
const ROLLS: usize = 256;

type Level = sapling_world::World;

/// Fires that tick: when next, and where.
#[derive(Resource, Default)]
pub(crate) struct Fires(Vec<(u64, Entity, BlockPos)>);

/// How readily fire catches by the block `name`, and how readily it burns away: the Java game's.
fn burns(name: &str) -> Option<[u32; 2]> {
    let bare = name.strip_prefix("minecraft:").unwrap_or(name);
    let tagged = |tag: &str| sapling_data::has_tag(name, tag);
    let ends = |ends: &[&str]| ends.iter().any(|end| bare.ends_with(end));
    let is_wooden = !["crimson", "warped", "nether_brick", "stone", "brick"].iter().any(|part| bare.contains(part));
    Some(match bare {
        _ if tagged("minecraft:logs_that_burn") || bare == "coal_block" => [5, 5],
        _ if is_wooden && (tagged("minecraft:planks") || tagged("minecraft:wooden_slabs")) => [5, 20],
        _ if is_wooden && ends(&["_fence", "_fence_gate", "_stairs"]) && sapling_data::has_tag(&name.replace("_fence_gate", "_planks").replace("_fence", "_planks").replace("_stairs", "_planks"), "minecraft:planks") => [5, 20],
        _ if ends(&["leaves", "_wool"]) || bare == "dried_kelp_block" => [30, 60],
        "bookshelf" | "lectern" => [30, 20],
        "tnt" => [15, 100],
        "vine" => [15, 100],
        "bamboo" | "scaffolding" => [60, 60],
        "hay_block" => [60, 20],
        _ if bare.ends_with("_carpet") && bare != "moss_carpet" => [60, 20],
        "short_grass" | "fern" | "tall_grass" | "large_fern" | "deadbush" | "poppy" | "dandelion" => [60, 100],
        _ => return None,
    })
}

fn name_at(level: &Level, [x, y, z]: BlockPos) -> Option<&str> {
    level.in_height(y).then(|| level.registry().get(level.block(x, y, z))).flatten().map(|state| state.name)
}

fn is_air(level: &Level, [x, y, z]: BlockPos) -> bool {
    level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_air())
}

fn is_solid(level: &Level, [x, y, z]: BlockPos) -> bool {
    level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_solid())
}

/// The readiest of the six blocks around `cell` to catch.
fn catches_by(level: &Level, cell: BlockPos) -> u32 {
    FACES.iter().filter_map(|face| burns(name_at(level, face.neighbour(cell))?)).map(|[catches, _]| catches).max().unwrap_or(0)
}

fn fire_of(level: &Level, age: u32) -> Option<u32> {
    let young = send::resolve(level.registry(), &Block::new(FIRE))?;
    with_property(level.registry(), young, AGE, &age.to_string())
}

/// Has the fire at `pos` tick in `after` ticks, or sooner if it was to.
fn schedule(world: &mut World, layer: Entity, pos: BlockPos, after: u64) {
    let when = world.resource::<Tick>().0 + after;
    let mut fires = world.get_resource_or_insert_with(Fires::default);
    match fires.0.iter_mut().find(|(_, in_layer, at)| (*in_layer, *at) == (layer, pos)) {
        Some(due) => due.0 = due.0.min(when),
        None => fires.0.push((when, layer, pos)),
    }
}

fn wait(world: &mut World) -> u64 {
    let [least, more] = EVERY;
    u64::from(least + world.get_resource_or_insert_with(Dice::default).roll(more))
}

/// A block was written at `pos`: fire, however it came there, ticks.
pub(super) fn written(world: &mut World, layer: Entity, pos: BlockPos) {
    if level(world, layer).and_then(|level| name_at(level, pos)) == Some(FIRE) {
        let wait = wait(world);
        schedule(world, layer, pos, wait);
    }
}

/// Whether fire may be at `pos`: over a solid block, or by one that burns.
pub(super) fn stands(level: &Level, [x, y, z]: BlockPos) -> bool {
    is_solid(level, [x, y - 1, z]) || catches_by(level, [x, y, z]) > 0
}

fn is_rained_on(level: &Level, [x, y, z]: BlockPos) -> bool {
    !(y + 1..).take_while(|y| level.in_height(*y)).any(|y| is_solid(level, [x, y, z]))
}

/// Lights the free cell `into`, over a solid block or by one that burns; false where no fire can be.
pub(super) fn light(world: &mut World, layer: Entity, into: BlockPos) -> bool {
    let [x, y, z] = into;
    let Some(level) = level(world, layer) else { return false };
    if !is_air(level, into) || !stands(level, into) {
        return false;
    }
    let is_soul = name_at(level, [x, y - 1, z]).is_some_and(|under| under.contains("soul_"));
    let Some(fire) = send::resolve(level.registry(), &Block::new(if is_soul { SOUL_FIRE } else { FIRE })) else { return false };
    send::set(world, layer, into, fire);
    if !is_soul {
        schedule(world, layer, into, 1);
    }
    true
}

/// What a fire's tick does.
#[derive(Default)]
struct Burnt {
    /// Whether the fire burns on, to tick again.
    burns_on: bool,
    /// The fire itself, older: no one is told, as BDS told no one of its fires' ages.
    aged: Option<u32>,
    set: Vec<(BlockPos, u32)>,
    primed: Vec<BlockPos>,
}

/// A tick of the fire at `pos`: the Java game's `FireBlock.tick`.
fn burnt(level: &Level, pos: BlockPos, hardship: u32, rains: bool, roll: &mut dyn FnMut(u32) -> u32) -> Burnt {
    let [x, y, z] = pos;
    let (mut done, air) = (Burnt::default(), level.registry().air_id());
    let Some(state) = level.registry().get(level.block(x, y, z)).filter(|state| state.name == FIRE) else { return done };
    let age: u32 = state.properties.split(',').filter_map(|pair| pair.split_once('=')).find(|(key, _)| *key == AGE).and_then(|(_, age)| age.parse().ok()).unwrap_or(0);
    let under = [x, y - 1, z];
    let (stands, is_fed) = (is_solid(level, under), catches_by(level, pos) > 0);
    let older = (age + roll(3) / 2).min(OLDEST);
    let burns_on = name_at(level, under).is_some_and(|under| ETERNAL.contains(&under));
    // Rain put BDS's fire over stone out within two seconds, and not its fire over netherrack.
    let goes_out = !(stands || is_fed) || !burns_on && rains && is_rained_on(level, pos) || !burns_on && if is_fed { older == OLDEST && roll(4) == 0 && name_at(level, under).and_then(burns).is_none() } else { !stands || older > 3 };
    if goes_out {
        done.set.push((pos, air));
        return done;
    }
    done.burns_on = true;
    done.aged = fire_of(level, older).filter(|_| older != age);
    let spread_age = |roll: &mut dyn FnMut(u32) -> u32| (older + roll(5) / 4).min(OLDEST);
    for face in FACES {
        let cell = face.neighbour(pos);
        let Some(name) = name_at(level, cell) else { continue };
        let Some([_, burns]) = burns(name) else { continue };
        if roll(if matches!(face, Face::Up | Face::Down) { OVER } else { BESIDE }) >= burns {
            continue;
        }
        match fire_of(level, spread_age(&mut *roll)).filter(|_| roll(older + 10) < 5) {
            _ if name == TNT => done.primed.push(cell),
            Some(fire) => done.set.push((cell, fire)),
            None => done.set.push((cell, air)),
        }
    }
    for [dx, dy, dz] in (-1..=1).flat_map(|dx| (-1..=4).flat_map(move |dy| (-1..=1).map(move |dz| [dx, dy, dz]))).filter(|by| *by != [0; 3]) {
        let cell = [x + dx, y + dy, z + dz];
        let catches = if is_air(level, cell) { catches_by(level, cell) } else { 0 };
        let sides = 100 + 100 * (dy - 1).max(0) as u32;
        let odds = (catches + 40 + hardship * 7) / (older + 30);
        if catches > 0 && odds > 0 && roll(sides) <= odds && !done.set.iter().any(|(at, _)| *at == cell)
            && let Some(fire) = fire_of(level, spread_age(&mut *roll))
        {
            done.set.push((cell, fire));
        }
    }
    done
}

/// Ticks the fires that are due.
pub(crate) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let Some(mut fires) = world.get_resource_mut::<Fires>().filter(|fires| !fires.0.is_empty()) else { return };
    let (due, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut fires.0).into_iter().partition(|(when, ..)| *when <= now);
    fires.0 = waiting;
    let hardship = match world.resource::<Settings>().0.difficulty {
        Difficulty::Peaceful => 0,
        Difficulty::Easy => 1,
        Difficulty::Normal => 2,
        Difficulty::Hard => 3,
    };
    for (_, layer, pos) in due {
        let rolls: Vec<u32> = {
            let mut dice = world.get_resource_or_insert_with(Dice::default);
            (0..ROLLS).map(|_| dice.roll(1 << 20)).collect()
        };
        let mut rolls = rolls.into_iter();
        let rains = crate::weather::of(world, layer) != Weather::Clear;
        let Some(level) = level(world, layer).filter(|level| level.level().rules.on(RULE)) else { continue };
        let burnt = burnt(level, pos, hardship, rains, &mut |sides| rolls.next().unwrap_or(0) % sides.max(1));
        let wait = wait(world);
        if let Some(aged) = burnt.aged {
            send::set_quietly(world, layer, pos, aged);
        }
        for (cell, id) in burnt.set {
            send::set(world, layer, cell, id);
        }
        for cell in burnt.primed {
            crate::tnt::light(world, layer, cell);
        }
        if burnt.burns_on {
            schedule(world, layer, pos, wait);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wood_and_wool_burn_and_stone_does_not() {
        let burning = ["minecraft:oak_planks", "minecraft:oak_log", "minecraft:white_wool", "minecraft:oak_leaves", "minecraft:stone", "minecraft:crimson_planks"].map(burns);
        assert_eq!(burning, [Some([5, 20]), Some([5, 5]), Some([30, 60]), Some([30, 60]), None, None]);
    }
}
