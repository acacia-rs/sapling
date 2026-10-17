//! What a piston takes along and breaks (docs/research/redstone.md, "Slime and honey").
// TODO: the blocks that stay are a list, of which BDS showed only obsidian; of the order slime's
// neighbours are taken in, BDS showed up, down and south for a push east.

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};

use super::{Pistons, block_name, is_extended, piston_at};
use crate::blocks::{level, send};

/// The most blocks a piston moves.
const LIMIT: usize = 12;
const STAYS: &[&str] = &[
    "obsidian", "bedrock", "barrier", "end_portal_frame", "end_portal", "portal", "end_gateway", "enchanting_table", "beacon", "mob_spawner", "respawn_anchor",
    "reinforced_deepslate", "moving_block", "piston_arm_collision", "command_block", "structure_block", "jigsaw",
];
const STICKY: [&str; 2] = ["slime", "honey_block"];
const SIDES: [Face; 6] = [Face::Up, Face::Down, Face::North, Face::South, Face::West, Face::East];

#[derive(PartialEq)]
enum Cell {
    Empty,
    Breaks,
    Moves,
    Stays,
}

/// The blocks one move of a piston takes along and breaks.
struct Plan<'w> {
    world: &'w World,
    layer: Entity,
    piston: BlockPos,
    step: Face,
    /// The arm's own cell as a piston draws back: empty by the time the blocks come.
    freed: Option<BlockPos>,
    taken: Vec<BlockPos>,
    broken: Vec<BlockPos>,
}

impl Plan<'_> {
    fn cell(&self, pos: BlockPos) -> Cell {
        let (world, layer) = (self.world, self.layer);
        if self.freed == Some(pos) {
            return Cell::Empty;
        }
        let Some(level) = level(world, layer).filter(|level| level.in_height(pos[1]) && level.is_loaded(send::column(pos))) else { return Cell::Stays };
        let Some(state) = level.registry().get(level.block(pos[0], pos[1], pos[2])) else { return Cell::Stays };
        let name = state.name.strip_prefix("minecraft:").unwrap_or(state.name);
        let is_out = || piston_at(world, layer, pos).is_some_and(|piston| is_extended(world, layer, pos, &piston));
        let is_busy = || world.get_resource::<Pistons>().is_some_and(|pistons| pistons.0.iter().any(|going| (going.layer, going.pos) == (layer, pos)));
        match () {
            _ if state.is_air() || state.is_liquid() => Cell::Empty,
            _ if pos == self.piston || STAYS.iter().any(|stays| name.ends_with(stays)) || is_out() || is_busy() => Cell::Stays,
            _ if !state.is_solid() => Cell::Breaks,
            _ => Cell::Moves,
        }
    }

    fn name(&self, pos: BlockPos) -> &'static str {
        block_name(self.world, self.layer, pos).map_or("", |(_, name, _)| name)
    }

    /// Whether the sticky block `holder` holds the block `held` beside it.
    fn holds(&self, holder: &str, held: BlockPos) -> bool {
        let name = self.name(held);
        self.cell(held) == Cell::Moves && !name.ends_with("glazed_terracotta") && !(STICKY.contains(&name) && name != holder)
    }

    /// Takes the block at `pos` with all it pushes and holds; false when something stays in
    /// the way or they are too many.
    fn take(&mut self, pos: BlockPos) -> bool {
        match self.cell(pos) {
            Cell::Empty => return true,
            Cell::Breaks => {
                if !self.broken.contains(&pos) {
                    self.broken.push(pos);
                }
                return true;
            }
            Cell::Stays => return false,
            Cell::Moves if self.taken.contains(&pos) => return true,
            Cell::Moves if self.taken.len() == LIMIT => return false,
            Cell::Moves => self.taken.push(pos),
        }
        if !self.take(self.step.neighbour(pos)) {
            return false;
        }
        let holder = self.name(pos);
        if !STICKY.contains(&holder) {
            return true;
        }
        let step = self.step;
        SIDES.iter().filter(|side| **side != step).map(|side| side.neighbour(pos)).all(|beside| !self.holds(holder, beside) || self.take(beside))
    }

    /// The cells taken as the block at `first` goes, farthest first, and those broken.
    fn from(mut self, first: BlockPos) -> Option<Lists> {
        if !self.take(first) {
            return None;
        }
        let step = self.step.offset();
        self.taken.sort_by_key(|pos| std::cmp::Reverse(pos.iter().zip(step).map(|(at, way)| at * way).sum::<i32>()));
        Some((self.taken, self.broken))
    }
}

/// The cells whose blocks a piston takes along, farthest first, and those whose blocks it breaks.
pub(super) type Lists = (Vec<BlockPos>, Vec<BlockPos>);

/// What the piston at `piston` pushing to `front` takes and breaks; `None` when something stays
/// in the way or they are too many.
pub(super) fn pushed(world: &World, layer: Entity, piston: BlockPos, front: Face) -> Option<Lists> {
    Plan { world, layer, piston, step: front, freed: None, taken: Vec::new(), broken: Vec::new() }.from(front.neighbour(piston))
}

/// What a sticky piston at `piston` drawing back to `back` brings with the block two cells out.
pub(super) fn drawn(world: &World, layer: Entity, piston: BlockPos, back: Face) -> Lists {
    let arm = [0, 1, 2].map(|axis| piston[axis] - back.offset()[axis]);
    let first = [0, 1, 2].map(|axis| arm[axis] - back.offset()[axis]);
    let plan = Plan { world, layer, piston, step: back, freed: Some(arm), taken: Vec::new(), broken: Vec::new() };
    if plan.cell(first) != Cell::Moves { Default::default() } else { plan.from(first).unwrap_or_default() }
}
