//! A bed used outside the Overworld bursts, as BDS's `BedBlock::use` (142cd8100): with
//! `respawnBlocksExplode` on, both halves go and a burning blast of 5 goes off at the head's
//! middle; off, the click does nothing (docs/research/blocks.md). The foot's dust goes first, at
//! its corner, whichever half was clicked (research/captures/bds-t3nether.txt, bds-x2bed.txt).

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
    let head = doors::bed_head(world, layer, pos).unwrap_or(pos);
    let at = head.map(|c| c as f32 + 0.5);
    let foot = [Some(pos), other].into_iter().flatten().find(|half| *half != head);
    let dusted = foot.unwrap_or(pos);
    if let Some(hash) = level(world, layer).map(|level| level.registry().network_hash(level.block(dusted[0], dusted[1], dusted[2])) as i32) {
        let [x, y, z] = dusted.map(|c| c as f32);
        send::to_viewers(world, layer, dusted, &[sapling_net::encode(&LevelEvent { event: LevelEventEvent::ParticleDestroyBlockNoSound, position: Vec3f { x, y, z }, data: hash })]);
    }
    // BDS lists the head's air first, then the foot's, with the blast's cells (bds-v2nether.txt, bds-x2bed.txt).
    let clears = move |world: &mut World| {
        for half in [Some(head), foot].into_iter().flatten() {
            update_block(world, layer, half, &Block::air());
        }
    };
    explode_burning(world, layer, at, POWER, Wreck::Decays, clears);
    true
}
