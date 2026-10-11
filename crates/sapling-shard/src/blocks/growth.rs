//! Random ticks for what grows: each crop, stem and berry bush has the chance a tick that a cell
//! picked at random around a player has on BDS, and grows a stage by the game's odds
//! (docs/research/growth.md).
// TODO: no light is asked for, where the game wants 9; a ripe stem bears no fruit; saplings,
// cane, cactus, grass, farmland, leaves, ice and the rest take no random tick.

use std::collections::HashSet;

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_data::RuleValue;
use sapling_world::ChunkPos;

use super::{level, send, with_property};

const RULE: &str = "randomTickSpeed";
/// Ticks between a cell's random ticks for each unit of the rule: BDS's lit ores lasted 1760 on
/// average and its wheat grew at the pace 1900 gives.
const EVERY: f32 = 1800.0;
const RIPE: u8 = 7;
const RIPE_BUSH: u8 = 3;
const FARMLAND: &str = "minecraft:farmland";
const GROWS: [&str; 6] = ["minecraft:wheat", "minecraft:carrots", "minecraft:potatoes", "minecraft:beetroot", "minecraft:melon_stem", "minecraft:pumpkin_stem"];
const BUSH: &str = "minecraft:sweet_berry_bush";

/// The cells that hold something that grows, by world: sampling every cell around the players
/// as BDS does cost more than a tick has when the rule is set high.
#[derive(Resource, Default)]
pub(crate) struct Growing(HashSet<(Entity, BlockPos)>);

type Level = sapling_world::World;

/// A xorshift of its own: the shard's dice are thrown by what happens, and what happens must not
/// turn on how many plants there are.
#[derive(Resource)]
pub(crate) struct Picks(u64);

impl Default for Picks {
    fn default() -> Self {
        Self(0x2545_F491_4F6C_DD1D)
    }
}

impl Picks {
    fn below(&mut self, sides: u32) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        ((self.0 >> 32) as u32) % sides.max(1)
    }
}

fn state_at(level: &Level, [x, y, z]: BlockPos) -> Option<(u32, &str, &str)> {
    let id = level.in_height(y).then(|| level.block(x, y, z))?;
    level.registry().get(id).map(|state| (id, state.name, state.properties))
}

fn number(properties: &str, key: &str) -> Option<u8> {
    properties.split(',').filter_map(|pair| pair.split_once('=')).find(|(name, _)| *name == key).and_then(|(_, value)| value.parse().ok())
}

/// How fast the crop `name` at `pos` grows: the Java game's reckoning, by the farmland under and
/// around it, wet or dry, and less in a field of its own kind that is not in rows.
fn speed(level: &Level, [x, y, z]: BlockPos, name: &str) -> f32 {
    let soil = |dx: i32, dz: i32| match state_at(level, [x + dx, y - 1, z + dz]) {
        Some((_, FARMLAND, properties)) => if number(properties, "moisturized_amount").is_some_and(|wet| wet > 0) { 3.0 } else { 1.0 },
        _ => 0.0,
    };
    let around: f32 = (-1..=1).flat_map(|dx| (-1..=1).map(move |dz| (dx, dz))).filter(|by| *by != (0, 0)).map(|(dx, dz)| soil(dx, dz) / 4.0).sum();
    let same = |dx: i32, dz: i32| state_at(level, [x + dx, y, z + dz]).is_some_and(|(_, beside, _)| beside == name);
    let (along, across) = (same(-1, 0) || same(1, 0), same(0, -1) || same(0, 1));
    let cornered = same(-1, -1) || same(1, -1) || same(1, 1) || same(-1, 1);
    let speed = 1.0 + soil(0, 0) + around;
    if along && across || cornered { speed / 2.0 } else { speed }
}

/// What a random tick makes of the block at `pos`.
fn grown(level: &Level, pos: BlockPos, picks: &mut Picks) -> Option<u32> {
    let (id, name, properties) = state_at(level, pos)?;
    let growth = number(properties, "growth")?;
    let grows = match name {
        BUSH => growth < RIPE_BUSH && picks.below(5) == 0,
        _ if GROWS.contains(&name) => growth < RIPE && picks.below((25.0 / speed(level, pos, name)) as u32 + 1) == 0,
        _ => false,
    };
    with_property(level.registry(), id, "growth", &(growth + 1).to_string()).filter(|_| grows)
}

fn is_growing(name: &str, properties: &str) -> bool {
    let ripe = if name == BUSH { RIPE_BUSH } else { RIPE };
    (name == BUSH || GROWS.contains(&name)) && number(properties, "growth").is_some_and(|growth| growth < ripe)
}

/// A block was written at `pos`: one that grows is kept track of.
pub(super) fn written(world: &mut World, layer: Entity, pos: BlockPos) {
    if level(world, layer).and_then(|level| state_at(level, pos)).is_some_and(|(_, name, properties)| is_growing(name, properties)) {
        world.get_resource_or_insert_with(Growing::default).0.insert((layer, pos));
    }
}

/// A column a world folder held came in: what grows in it is kept track of.
pub(crate) fn arrived(world: &mut World, layer: Entity, column: ChunkPos) {
    let Some(level) = level(world, layer) else { return };
    let registry = level.registry();
    let found = level.find_blocks(column, |id| registry.get(id).is_some_and(|state| is_growing(state.name, state.properties)));
    if !found.is_empty() {
        world.get_resource_or_insert_with(Growing::default).0.extend(found.into_iter().map(|pos| (layer, pos)));
    }
}

/// A tick's random ticks of everything that grows.
pub(crate) fn tick(world: &mut World) {
    let Some(growing) = world.get_resource::<Growing>().filter(|growing| !growing.0.is_empty()).map(|growing| growing.0.iter().copied().collect::<Vec<_>>()) else { return };
    let mut picks = world.remove_resource::<Picks>().unwrap_or_default();
    let (mut changed, mut done): (Vec<(Entity, BlockPos, u32)>, Vec<(Entity, BlockPos)>) = (Vec::new(), Vec::new());
    for (layer, pos) in growing {
        let Some(level) = level(world, layer) else { continue };
        if !state_at(level, pos).is_some_and(|(_, name, properties)| is_growing(name, properties)) {
            done.push((layer, pos));
            continue;
        }
        let Some((_, RuleValue::Int(rule))) = level.level().rules.get(RULE) else { continue };
        if (picks.below(1 << 20) as f32) < rule.max(0) as f32 / EVERY * (1 << 20) as f32 {
            changed.extend(grown(level, pos, &mut picks).map(|id| (layer, pos, id)));
        }
    }
    world.insert_resource(picks);
    let mut growing = world.resource_mut::<Growing>();
    for cell in done {
        growing.0.remove(&cell);
    }
    for (layer, pos, id) in changed {
        send::set(world, layer, pos, id);
    }
}
