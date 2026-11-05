//! Potent sulfur's eruptions, BDS's `PotentSulfurBlock` tick (Linux 0xcf57af0, docs/research/world-ticks.md
//! "Potent sulfur"): under a water source and over magma it lies dormant and erupts by turns, each wait
//! drawn from the cell's positional random and the height of the water over it.

use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::World as Level;
use sapling_worldgen::random::Xoroshiro;

use super::queued::{self, Job};
use super::{level, send, with_property};

const SULFUR: &str = "minecraft:potent_sulfur";
const STATE: &str = "potent_sulfur_state";
/// `potent_sulfur_state` by BDS's number for it.
const STATES: [&str; 5] = ["dry", "wet", "dormant", "erupting", "continuous"];
const DORMANT: usize = 2;
const ERUPTING: usize = 3;
const CONTINUOUS: usize = 4;
/// Seconds erupting and seconds dormant, lowest and highest, by the height of the water over it,
/// 1 to 4 (the table at 0x1c7debc).
const WAITS: [([f32; 2], [f32; 2]); 4] = [([1.0, 2.0], [15.0, 30.0]), ([2.0, 3.0], [25.0, 40.0]), ([3.0, 4.0], [35.0, 50.0]), ([4.0, 5.0], [45.0, 60.0])];

/// The state number of the potent sulfur at `pos`; `None` for another block.
fn state_at(level: &Level, [x, y, z]: BlockPos) -> Option<usize> {
    let state = level.registry().get(level.block(x, y, z)).filter(|state| state.name == SULFUR)?;
    STATES.iter().position(|&name| Some(name) == state.property(STATE))
}

fn is_source(level: &Level, id: u32, water: bool) -> bool {
    level.registry().get(id).is_some_and(|state| state.property("liquid_depth") == Some("0") && if water { state.is_water() } else { state.is_lava() })
}

/// A water source fills the cell: as its block, or held beside a block with no collision box.
fn holds_water(level: &Level, [x, y, z]: BlockPos) -> bool {
    let block = level.block(x, y, z);
    is_source(level, block, true) || (is_source(level, level.liquid(x, y, z), true) && level.registry().get(block).is_some_and(|state| state.boxes.is_empty()))
}

/// How many cells of water stand over `pos`, up to 4 (0xcf59030).
fn water_height(level: &Level, [x, y, z]: BlockPos) -> usize {
    (1..=4).take_while(|&up| holds_water(level, [x, y + up, z])).count()
}

/// The state the cell's surroundings ask for (0xcf57e50): dry without water over it; over a lava source
/// continuous, over magma dormant (an eruption keeps on), else wet.
fn wanted(level: &Level, [x, y, z]: BlockPos, now: usize) -> usize {
    if !holds_water(level, [x, y + 1, z]) {
        return 0;
    }
    let below = level.block(x, y - 1, z);
    if is_source(level, below, false) {
        CONTINUOUS
    } else if level.registry().get(below).is_some_and(|state| state.name == "minecraft:magma") {
        if now == ERUPTING { ERUPTING } else { DORMANT }
    } else {
        1
    }
}

/// The water over `pos` if the cell over that column lets an eruption out (0xcf59340): neither layer a
/// liquid source and no collision box; 0 otherwise.
fn open_height(level: &Level, [x, y, z]: BlockPos) -> usize {
    let height = water_height(level, [x, y, z]);
    let top = y + height as i32 + 1;
    let registry = level.registry();
    let is_any_source = |id: u32| registry.get(id).is_some_and(|state| state.is_liquid() && state.property("liquid_depth") == Some("0"));
    let (block, held) = (level.block(x, top, z), level.liquid(x, top, z));
    let open = !is_any_source(block) && !is_any_source(held) && registry.get(block).is_some_and(|state| state.boxes.is_empty());
    if open { height } else { 0 }
}

/// Ticks to wait in `state` (2 or 3): seconds lerped by the cell's positional random from the level seed,
/// the first float for dormant, the second for erupting, times 20 and truncated.
fn wait(seed: i64, [x, y, z]: BlockPos, height: usize, state: usize) -> u64 {
    let (erupting, dormant) = WAITS[height.clamp(1, 4) - 1];
    // BDS xors the seed with 0x6a09e66739a12823 where Java's upgradeSeedTo128bit (from_seed) uses its silver ratio.
    let upgraded = seed as u64 ^ 0x6a09_e667_39a1_2823 ^ 0x6a09_e667_f3bc_c909;
    let mut random = Xoroshiro::from_seed(upgraded as i64).fork_positional().at(x, y, z);
    let first = random.next_float();
    let [low, high] = if state == DORMANT { dormant } else { erupting };
    let u = if state == DORMANT { first } else { random.next_float() };
    (((high - low) * u + low) * 20.0) as i32 as u64
}

/// Sets the cell to `state` (0xcf57f80), heard when an eruption starts with water over it open to the
/// top; dormant or erupting queues its next turn even when the state is already so.
fn set(world: &mut World, layer: Entity, pos: BlockPos, now: usize, state: usize) {
    let Some(level) = level(world, layer) else { return };
    let (height, open) = (water_height(level, pos), open_height(level, pos));
    if state != now {
        let Some(id) = with_property(level.registry(), level.block(pos[0], pos[1], pos[2]), STATE, STATES[state]) else { return };
        send::set(world, layer, pos, id);
        if state >= ERUPTING && now < ERUPTING && open > 0 {
            let heard = if state == CONTINUOUS { "geyser_continuous_eruption_start" } else { "geyser_eruption_start" };
            let [x, y, z] = pos.map(|c| c as f32 + 0.5);
            send::to_viewers(world, layer, pos, &[send::noise(heard, Vec3f { x, y, z }, -1)]);
        }
    }
    if state == DORMANT || state == ERUPTING {
        let seed = world.get::<crate::components::Layer>(crate::dimensions::home(world, layer)).map_or(0, |home| home.world.level().seed);
        queued::after(world, layer, pos, wait(seed, pos, height, state), Job::PotentSulfur);
    }
}

/// The potent sulfur at `pos` takes what its surroundings ask for, or, asked for what it is, turns from
/// dormant to erupting or back.
pub(super) fn ticked(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(level) = level(world, layer) else { return };
    let Some(now) = state_at(level, pos) else { return };
    let next = match wanted(level, pos, now) {
        asked if asked != now => asked,
        DORMANT => ERUPTING,
        ERUPTING => DORMANT,
        _ => return,
    };
    set(world, layer, pos, now, next);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waits_are_the_ones_bds_queued_on_the_drill_world() {
        // sulfur_gdb2.log on af2load: seed 0xb55efc8fb0f7e382, cell 6 -21 13 under one water.
        let seed = 0xb55e_fc8f_b0f7_e382_u64 as i64;
        assert_eq!(wait(seed, [6, -21, 13], 1, DORMANT), 399);
        assert_eq!(wait(seed, [6, -21, 13], 1, ERUPTING), 37);
    }
}
