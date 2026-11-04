//! A bed used outside the Overworld bursts, as BDS's `BedBlock::use` (142cd8100): with
//! `respawnBlocksExplode` on, both halves go and a burning blast of 5 goes off at the head's
//! middle; off, the click does nothing (docs/research/blocks.md). The clicked half's dust goes
//! first, at its corner, the foot's too (research/captures/bds-t3nether.txt).

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{Block, BlockPos};

use super::{doors, level, put_settled as update_block, send};
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
    if let Some(hash) = level(world, layer).map(|level| level.registry().network_hash(level.block(pos[0], pos[1], pos[2])) as i32) {
        let [x, y, z] = pos.map(|c| c as f32);
        send::to_viewers(world, layer, pos, &[sapling_net::encode(&LevelEvent { event: LevelEventEvent::ParticleDestroyBlockNoSound, position: Vec3f { x, y, z }, data: hash })]);
    }
    update_block(world, layer, pos, &Block::air());
    if let Some(other) = other {
        update_block(world, layer, other, &Block::air());
    }
    explode_burning(world, layer, at, POWER, Wreck::Decays);
    true
}
