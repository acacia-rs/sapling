//! A bed used outside the Overworld bursts, as BDS's `BedBlock::use` (142cd8100): with
//! `respawnBlocksExplode` on, both halves go and a burning blast of 5 goes off at the head's
//! middle; off, the click does nothing (docs/research/blocks.md).

use bevy_ecs::prelude::*;
use sapling_api::{Block, BlockPos};

use super::{doors, level, update_block};
use crate::creatures::{Wreck, explode_burning};

const OVERWORLD: i32 = 0;
const POWER: f32 = 5.0;
const BURSTS: &str = "respawnBlocksExplode";

/// A click on the bed at `pos`: false where beds work; else it bursts, if the world lets it.
pub(crate) fn bursts(world: &mut World, layer: Entity, pos: BlockPos) -> bool {
    if level(world, layer).is_none_or(|level| level.dimension() == OVERWORLD) {
        return false;
    }
    if !crate::rules::on(world, layer, BURSTS) {
        return true;
    }
    let other = doors::other_half(world, layer, pos);
    let at = doors::bed_head(world, layer, pos).unwrap_or(pos).map(|c| c as f32 + 0.5);
    update_block(world, layer, pos, &Block::air());
    if let Some(other) = other {
        update_block(world, layer, other, &Block::air());
    }
    explode_burning(world, layer, at, POWER, Wreck::Decays);
    true
}
