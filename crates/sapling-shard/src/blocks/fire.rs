//! Fire: flint and steel or a fire charge lights it over a solid block or by one that burns, soul
//! fire over soul sand; it ages, spreads to what burns, burns that away and goes out by BDS's rule
//! (burning.rs, docs/research/fire.md). Every fire put down, by hand, command, lava or lightning,
//! ticks a step on and then every 30 to 39 steps of its world's random-tick queue
//! (random_queue.rs), which runs `randomTickSpeed` steps a game tick; one a world folder holds
//! ticks when its column kept its tick (pending.rs), as on BDS.

mod burning;

use bevy_ecs::prelude::*;
use sapling_api::{Block, BlockPos, Difficulty, Face};

use super::{Dice, level, random_queue, send};
use crate::components::Settings;
use burning::{Around, FIRE, SOUL_FIRE, burns, fire_of, is_air, name_at};

pub(crate) use burning::stands;

/// A fire's ticks are this many steps apart, and up to this many more (147157ed0); a fire put
/// down ticks a step on (its place handler 1471579c0).
const EVERY: [u32; 2] = [30, 10];
const FIRST: u64 = 1;
const RULE: &str = "doFireTick";
const ROLLS: usize = 256;
const CHANCES: u32 = 1 << 20;

/// Fires that tick: when next on their world's random-tick queue, and where.
#[derive(Resource, Default)]
pub(crate) struct Fires(Vec<(u64, Entity, BlockPos)>);

/// Has the fire at `pos` tick `after` steps on, or sooner if it was to.
fn schedule(world: &mut World, layer: Entity, pos: BlockPos, after: u64) {
    let when = random_queue::now(world, layer) + after;
    let mut fires = world.get_resource_or_insert_with(Fires::default);
    match fires.0.iter_mut().find(|(_, in_layer, at)| (*in_layer, *at) == (layer, pos)) {
        Some(due) => due.0 = due.0.min(when),
        None => fires.0.push((when, layer, pos)),
    }
}

/// What waits, for its column to keep (pending.rs): each fire and the game tick it ticks at the
/// queue's speed now.
pub(super) fn waiting(world: &World) -> Vec<(Entity, BlockPos, u64)> {
    let now = world.resource::<crate::components::Tick>().0;
    world.get_resource::<Fires>().map(|fires| fires.0.iter().map(|&(when, layer, pos)| (layer, pos, now + random_queue::ticks_until(world, layer, when))).collect()).unwrap_or_default()
}

/// Drops what waits where `saved` says its column kept it and is gone.
pub(super) fn forget(world: &mut World, saved: &dyn Fn(Entity, BlockPos) -> bool) {
    if let Some(mut fires) = world.get_resource_mut::<Fires>() {
        fires.0.retain(|&(_, layer, pos)| !saved(layer, pos));
    }
}

/// Has the fire at `pos` tick in `delay` game ticks at the queue's speed now, as its column kept it.
pub(super) fn again(world: &mut World, layer: Entity, pos: BlockPos, delay: u64) {
    let steps = delay.max(1) * random_queue::speed(world, layer).max(1);
    schedule(world, layer, pos, steps);
}

fn wait(world: &mut World) -> u64 {
    let [least, more] = EVERY;
    u64::from(least + world.get_resource_or_insert_with(Dice::default).roll(more))
}

/// A block that `was` something else was written at `pos`: fire, however it came there, ticks a step on.
/// A fire aging is no new fire: rescheduling it then put fires over stone out in 3 s at rule 4 against
/// BDS's 5.9 (sapling-firepace.txt at 5ba26c16).
pub(super) fn written(world: &mut World, layer: Entity, pos: BlockPos, was: &str) {
    if was != FIRE && level(world, layer).and_then(|level| name_at(level, pos)) == Some(FIRE) {
        schedule(world, layer, pos, FIRST);
    }
}

/// Lights the free cell `into`, over a solid block or by one that burns; false where no fire can be.
pub(crate) fn light(world: &mut World, layer: Entity, into: BlockPos) -> bool {
    let [x, y, z] = into;
    if !level(world, layer).is_some_and(|level| is_air(level, into)) {
        return false;
    }
    // In a whole frame the spark lights a portal and no fire is placed (BDS 141a79fc0).
    if super::portals::kindle(world, layer, into) {
        return true;
    }
    let Some(level) = level(world, layer).filter(|level| stands(level, into)) else { return false };
    let is_soul = name_at(level, [x, y - 1, z]).is_some_and(|under| under.contains("soul_"));
    let Some(fire) = send::resolve(level.registry(), &Block::new(if is_soul { SOUL_FIRE } else { FIRE })) else { return false };
    send::set(world, layer, into, fire);
    true
}

/// Whether lava under `over` may light fire on its random tick: air or what burns is over it (the
/// lavafire drill's lava under a plank lit the air beside the plank).
pub(super) fn lights_under(level: &sapling_world::World, over: BlockPos) -> bool {
    is_air(level, over) || name_at(level, over).and_then(burns).is_some()
}

/// A random tick of the lava at `pos`: with air or what burns over it, up to ten cells around it
/// are tried, and the first that burns sets fire to the first air beside it, +x, -x, +y, -y, +z, -z
/// (`LiquidBlock::randomTick` 148b5cd10, not the Java game's rule).
pub(super) fn kindled_by_lava(world: &mut World, layer: Entity, pos: BlockPos, roll: &mut impl FnMut(u32) -> u32) {
    const TRIES: usize = 10;
    const BESIDE: [BlockPos; 6] = [[1, 0, 0], [-1, 0, 0], [0, 1, 0], [0, -1, 0], [0, 0, 1], [0, 0, -1]];
    if !crate::rules::on(world, layer, RULE) {
        return;
    }
    let Some(level) = level(world, layer).filter(|level| lights_under(level, Face::Up.neighbour(pos))) else { return };
    let flammable = |cell: BlockPos| name_at(level, cell).and_then(burns).is_some();
    for _ in 0..TRIES {
        let tried: BlockPos = std::array::from_fn(|axis| pos[axis] + roll(3) as i32 - 1);
        if !flammable(tried) {
            continue;
        }
        let Some(air) = BESIDE.iter().map(|step| std::array::from_fn(|axis| tried[axis] + step[axis])).find(|cell| is_air(level, *cell)) else { continue };
        if let Some(fire) = fire_of(level, 0) {
            send::set(world, layer, air, fire);
        }
        return;
    }
}

/// Ticks the fires due on their world's random-tick queue; one whose column does not tick waits as
/// many steps more as the queue runs a tick ([`crate::ticking::Region::hold`]).
pub(crate) fn tick(world: &mut World) {
    let Some(mut queue) = world.get_resource_mut::<Fires>().filter(|fires| !fires.0.is_empty()).map(|mut fires| std::mem::take(&mut fires.0)) else { return };
    crate::ticking::Region::of(world).hold(world, &mut queue, |layer| random_queue::speed(world, layer));
    let mut layers: Vec<Entity> = queue.iter().map(|(_, layer, _)| *layer).collect();
    layers.sort_unstable();
    layers.dedup();
    let clocks: Vec<(Entity, u64)> = layers.into_iter().map(|layer| (layer, random_queue::now(world, layer))).collect();
    let is_due = |when: u64, layer: Entity| clocks.iter().any(|&(at, now)| at == layer && when <= now);
    let (due, waiting): (Vec<_>, Vec<_>) = queue.into_iter().partition(|&(when, layer, _)| is_due(when, layer));
    world.resource_mut::<Fires>().0.extend(waiting);
    let hardship = match world.resource::<Settings>().0.difficulty {
        Difficulty::Peaceful => 0,
        Difficulty::Easy => 1,
        Difficulty::Normal => 2,
        Difficulty::Hard => 3,
    };
    for (_, layer, pos) in due {
        let rolls: Vec<u32> = {
            let mut dice = world.get_resource_or_insert_with(Dice::default);
            (0..2 * ROLLS).map(|_| dice.roll(CHANCES)).collect()
        };
        let (ints, floats) = rolls.split_at(ROLLS);
        let (mut ints, mut floats) = (ints.iter(), floats.iter());
        let around = Around { hardship, rains: crate::weather::rains(world, layer), spreads: crate::rules::on(world, layer, RULE) };
        let Some(level) = level(world, layer) else { continue };
        let mut roll = |sides: u32| ints.next().copied().unwrap_or(0) % sides.max(1);
        let mut chance = || floats.next().copied().unwrap_or(0) as f32 / CHANCES as f32;
        let burnt = burning::burnt(level, pos, &around, &mut roll, &mut chance);
        let wait = wait(world);
        if let Some(aged) = burnt.aged {
            send::set_quietly(world, layer, pos, aged);
        }
        for (cell, id) in burnt.set {
            let burns_away = cell != pos && super::level(world, layer).is_some_and(|level| !is_air(level, cell));
            // The fire going out at `pos` is no spread.
            if (cell == pos || send::may_spread(world, layer, cell, pos, |_| Some(id))) && send::set(world, layer, cell, id) && burns_away {
                // What stood on a burnt block falls or breaks, as BDS's did (bds-burnfloat.txt).
                super::updates::settle(world, layer, cell);
            }
        }
        for cell in burnt.primed {
            crate::tnt::light(world, layer, cell);
        }
        if burnt.burns_on {
            schedule(world, layer, pos, wait);
        }
    }
}
