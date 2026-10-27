//! Random ticks for what grows, dries, decays and melts: each crop, stem, berry bush, cane, cactus,
//! bamboo, farmland, open dirt, smothered grass, lit ice and snow and marked leaf has the chance a
//! tick that a cell picked at random around a player has on BDS (docs/research/growth.md); what a
//! tick then does is in rules.rs and cover.rs, and snowfall.rs freezes water and lays snow.
// A sapling whose tick comes up grows in saplings.rs; mushrooms, vines, cave vines, stalactites
// and lava draw theirs (sampled.rs, wild.rs, vines.rs, drips.rs, fire.rs); kelp keeps its own (kelp.rs).
// TODO: only a survival player treads farmland down, no mob and no creative player.

mod copper;
mod cover;
mod kelp;
mod rules;
mod sampled;
mod snowfall;
pub(super) mod vines;
mod wild;

use std::collections::HashSet;

use bevy_ecs::prelude::*;
use sapling_api::{Block, BlockPos};
use sapling_data::RuleValue;
use sapling_world::ChunkPos;

use super::{level, send};

pub(super) use kelp::meal as meal_kelp;

const RULE: &str = "randomTickSpeed";
/// Ticks between a cell's random ticks for each unit of the rule: BDS's lit ores lasted 1760 on
/// average and its wheat grew at the pace 1900 gives.
const EVERY: f32 = 1800.0;
const CHANCES: u32 = 1 << 20;
const DIRT: &str = "minecraft:dirt";
const FARMLAND: &str = "minecraft:farmland";
/// A drop of this much treads no farmland, and the feet are less than this over the cell trodden.
const SAFE_DROP: f32 = 0.5;
const TROD_UNDER: f32 = 0.2;

/// The cells that hold something a random tick changes, by world: sampling every cell around
/// the players as BDS does cost more than a tick has when the rule is set high.
#[derive(Resource, Default)]
pub(crate) struct Growing(HashSet<(Entity, BlockPos)>);

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

/// The name of the block at `pos`, if it is one kept track of.
fn kept(level: &sapling_world::World, [x, y, z]: BlockPos) -> Option<&str> {
    let name = level.in_height(y).then(|| level.registry().get(level.block(x, y, z))).flatten().filter(|state| rules::is_ticked(state.name, state.properties)).map(|state| state.name)?;
    (!cover::is_ticked(name) || cover::is_kept(level, [x, y, z], name)).then_some(name)
}

fn ticked_at(world: &World, layer: Entity, pos: BlockPos) -> Option<&str> {
    kept(level(world, layer)?, pos)
}

/// Whether the block `name` with `properties` is one whose random ticks are drawn (sampled.rs):
/// wild plants, stalactites and lava with air over it (BDS's also ticks lava under what burns).
fn is_sampled(name: &str, properties: &str) -> bool {
    wild::is_ticked(name) || name == "minecraft:pointed_dripstone" && properties.contains("hanging=1") || matches!(name, "minecraft:lava" | "minecraft:flowing_lava")
}

fn sampled_at(level: &sapling_world::World, [x, y, z]: BlockPos) -> Option<&'static str> {
    let state = level.in_height(y).then(|| level.registry().get(level.block(x, y, z))).flatten().filter(|state| is_sampled(state.name, state.properties))?;
    let is_lava = state.name.ends_with("lava");
    let open = || level.in_height(y + 1) && level.registry().get(level.block(x, y + 1, z)).is_some_and(|over| over.is_air());
    (!is_lava || open()).then_some(state.name)
}

/// A block was written at `pos`: one a random tick changes is kept track of, and one that
/// shines enough to melt has the ice and snow near it looked at from the next tick, when its
/// light is in.
pub(super) fn written(world: &mut World, layer: Entity, pos: BlockPos) {
    // And the cell under it: dirt a block was taken off is open now, grass a block was put on is not.
    let mut cells: Vec<BlockPos> = [pos, [pos[0], pos[1] - 1, pos[2]]].into_iter().filter(|cell| ticked_at(world, layer, *cell).is_some()).collect();
    let mut drawn = Vec::new();
    if let Some(level) = level(world, layer) {
        cells.extend(cover::melted_by(level, pos));
        drawn.extend([pos, [pos[0], pos[1] - 1, pos[2]]].into_iter().filter(|cell| sampled_at(level, *cell).is_some()));
    }
    if !cells.is_empty() {
        world.get_resource_or_insert_with(Growing::default).0.extend(cells.into_iter().map(|cell| (layer, cell)));
    }
    if !drawn.is_empty() {
        sampled::keep(world, layer, drawn);
    }
    kelp::written(world, layer, pos);
}

/// A column a world folder held came in: what random ticks change in it is kept track of.
pub(crate) fn arrived(world: &mut World, layer: Entity, column: ChunkPos) {
    let Some(level) = level(world, layer) else { return };
    let registry = level.registry();
    let mut found = level.find_blocks(column, |id| registry.get(id).is_some_and(|state| rules::is_ticked(state.name, state.properties)));
    found.retain(|pos| kept(level, *pos).is_some());
    let mut drawn = level.find_blocks(column, |id| registry.get(id).is_some_and(|state| is_sampled(state.name, state.properties)));
    drawn.retain(|pos| sampled_at(level, *pos).is_some());
    let kelp = level.find_blocks(column, |id| registry.get(id).is_some_and(|state| state.name == "minecraft:kelp"));
    if !found.is_empty() {
        world.get_resource_or_insert_with(Growing::default).0.extend(found.into_iter().map(|pos| (layer, pos)));
    }
    if !drawn.is_empty() {
        sampled::keep(world, layer, drawn);
    }
    kelp::arrived(world, layer, &kelp);
}

/// The random ticks of the cells drawn rather than rolled (sampled.rs).
fn draw_ticks(world: &mut World, picks: &mut Picks) {
    for layer in sampled::layers(world) {
        let Some((_, RuleValue::Int(rule))) = level(world, layer).and_then(|level| level.level().rules.get(RULE)) else { continue };
        let chance = rule.max(0) as f32 / EVERY;
        for pos in sampled::picked(world, layer, chance, picks) {
            let Some(name) = level(world, layer).and_then(|level| sampled_at(level, pos)) else {
                sampled::forget(world, layer, pos);
                continue;
            };
            match name {
                "minecraft:pointed_dripstone" => super::drips::ticked(world, layer, pos, &mut || picks.below(1 << 24) as f32 / (1 << 24) as f32),
                "minecraft:lava" | "minecraft:flowing_lava" => super::fire::kindled_by_lava(world, layer, pos, &mut |sides| picks.below(sides)),
                _ => wild::ticked(world, layer, pos, name, picks),
            }
        }
    }
}

/// A body came down at `feet` after falling `dropped` blocks: farmland under it is trodden to
/// dirt, likelier the further it fell (the Java game's chance; BDS's stood a drop of 0.8 three
/// times and went under 1.5 and 3).
pub(crate) fn trample(world: &mut World, layer: Entity, feet: [f32; 3], dropped: f32) {
    // Farmland is lower than a block: the cell under the feet, not the one they are in.
    let pos = [feet[0].floor() as i32, (feet[1] - TROD_UNDER).floor() as i32, feet[2].floor() as i32];
    if dropped <= SAFE_DROP || kept(match level(world, layer) { Some(level) => level, None => return }, pos) != Some(FARMLAND) {
        return;
    }
    let mut picks = world.remove_resource::<Picks>().unwrap_or_default();
    let treads = (picks.below(CHANCES) as f32) < (dropped - SAFE_DROP) * CHANCES as f32;
    world.insert_resource(picks);
    if treads {
        super::update_block(world, layer, pos, &Block::new(DIRT));
    }
}

/// A tick's random ticks of everything kept track of.
pub(crate) fn tick(world: &mut World) {
    let mut picks = world.remove_resource::<Picks>().unwrap_or_default();
    snowfall::tick(world, &mut picks);
    draw_ticks(world, &mut picks);
    kelp::tick(world, &mut picks);
    let Some(growing) = world.get_resource::<Growing>().filter(|growing| !growing.0.is_empty()).map(|growing| growing.0.iter().copied().collect::<Vec<_>>()) else {
        world.insert_resource(picks);
        return;
    };
    let (mut changed, mut done): (Vec<(Entity, BlockPos, bool, Vec<rules::Change>)>, Vec<(Entity, BlockPos)>) = (Vec::new(), Vec::new());
    let mut saplings = Vec::new();
    for (layer, pos) in growing {
        let Some(name) = ticked_at(world, layer, pos) else {
            done.push((layer, pos));
            continue;
        };
        let Some(level) = level(world, layer) else { continue };
        let Some((_, RuleValue::Int(rule))) = level.level().rules.get(RULE) else { continue };
        let chance = rule.max(0) as f32 * rules::haste(name) / EVERY;
        if (picks.below(CHANCES) as f32) >= chance * CHANCES as f32 {
            continue;
        }
        if super::saplings::is_ticked(name.strip_prefix("minecraft:").unwrap_or(name), "") {
            if picks.below(super::saplings::TICK_ODDS) == 0 {
                saplings.push((layer, pos));
            }
        } else {
            changed.push((layer, pos, rules::grows(name), rules::ticked(level, pos, crate::light::darken(world, layer), &mut picks)));
        }
    }
    world.insert_resource(picks);
    let mut growing = world.resource_mut::<Growing>();
    for cell in done {
        growing.0.remove(&cell);
    }
    for (layer, ticked, grows, changes) in changed {
        if changes.is_empty() || grows && !may_grow(world, layer, ticked) {
            continue;
        }
        for rules::Change { pos, id, how } in changes {
            match how {
                rules::How::Breaks => super::updates::pop(world, layer, pos),
                rules::How::Quiet => _ = send::set_quietly(world, layer, pos, id),
                rules::How::Shown => _ = send::set(world, layer, pos, id),
                rules::How::Melts => _ = super::liquid::pour(world, layer, pos, super::liquid::Liquid::Water, false),
            }
        }
    }
    for (layer, pos) in saplings {
        if may_grow(world, layer, pos) {
            super::saplings::ticked(world, layer, pos);
        }
    }
}

/// Whether plugins let the plant at `pos` grow ([`sapling_api::BlockGrowEvent`]).
fn may_grow(world: &mut World, layer: Entity, pos: BlockPos) -> bool {
    !crate::plugins::stopped_in(world, layer, |world| sapling_api::BlockGrowEvent::new(pos, send::block_at(world, layer, pos).unwrap_or_else(Block::air)))
}
