//! Random ticks for what grows and dries: each crop, stem, berry bush, cane, cactus, bamboo and
//! farmland has the chance a tick that a cell picked at random around a player has on BDS
//! (docs/research/growth.md); what a tick then does is in rules.rs.
// TODO: no light is asked for, where the game wants 9; saplings grow no tree; grass, leaves, ice
// and the rest take no random tick.

mod rules;

use std::collections::HashSet;

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_data::RuleValue;
use sapling_world::ChunkPos;

use super::{level, send};

const RULE: &str = "randomTickSpeed";
/// Ticks between a cell's random ticks for each unit of the rule: BDS's lit ores lasted 1760 on
/// average and its wheat grew at the pace 1900 gives.
const EVERY: f32 = 1800.0;
const CHANCES: u32 = 1 << 20;

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

fn ticked_at(world: &World, layer: Entity, [x, y, z]: BlockPos) -> Option<&str> {
    let level = level(world, layer).filter(|level| level.in_height(y))?;
    level.registry().get(level.block(x, y, z)).filter(|state| rules::is_ticked(state.name, state.properties)).map(|state| state.name)
}

/// A block was written at `pos`: one a random tick changes is kept track of.
pub(super) fn written(world: &mut World, layer: Entity, pos: BlockPos) {
    if ticked_at(world, layer, pos).is_some() {
        world.get_resource_or_insert_with(Growing::default).0.insert((layer, pos));
    }
}

/// A column a world folder held came in: what random ticks change in it is kept track of.
pub(crate) fn arrived(world: &mut World, layer: Entity, column: ChunkPos) {
    let Some(level) = level(world, layer) else { return };
    let registry = level.registry();
    let found = level.find_blocks(column, |id| registry.get(id).is_some_and(|state| rules::is_ticked(state.name, state.properties)));
    if !found.is_empty() {
        world.get_resource_or_insert_with(Growing::default).0.extend(found.into_iter().map(|pos| (layer, pos)));
    }
}

/// A tick's random ticks of everything kept track of.
pub(crate) fn tick(world: &mut World) {
    let Some(growing) = world.get_resource::<Growing>().filter(|growing| !growing.0.is_empty()).map(|growing| growing.0.iter().copied().collect::<Vec<_>>()) else { return };
    let mut picks = world.remove_resource::<Picks>().unwrap_or_default();
    let (mut changed, mut done): (Vec<(Entity, rules::Change)>, Vec<(Entity, BlockPos)>) = (Vec::new(), Vec::new());
    for (layer, pos) in growing {
        let Some(name) = ticked_at(world, layer, pos) else {
            done.push((layer, pos));
            continue;
        };
        let Some(level) = level(world, layer) else { continue };
        let Some((_, RuleValue::Int(rule))) = level.level().rules.get(RULE) else { continue };
        let chance = rule.max(0) as f32 * rules::haste(name) / EVERY;
        if (picks.below(CHANCES) as f32) < chance * CHANCES as f32 {
            changed.extend(rules::ticked(level, pos, &mut picks).into_iter().map(|change| (layer, change)));
        }
    }
    world.insert_resource(picks);
    let mut growing = world.resource_mut::<Growing>();
    for cell in done {
        growing.0.remove(&cell);
    }
    for (layer, rules::Change { pos, id, quiet }) in changed {
        if quiet {
            send::set_quietly(world, layer, pos, id);
        } else {
            send::set(world, layer, pos, id);
        }
    }
}
