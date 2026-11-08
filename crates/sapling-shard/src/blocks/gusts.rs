//! What a wind burst does to the blocks its rays reach (creatures/charges.rs): each door, trapdoor,
//! fence gate and lever swings, each button goes in, as by a hand, a door once for both halves
//! (research/captures/bds-d3breeze.txt: wooden ones moved, an iron door did not).

use std::collections::BTreeSet;

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::{doors, level};

/// The cells a gust reached are swung, the tick after its burst as BDS's were.
pub(crate) fn gusted(world: &mut World, layer: Entity, cells: &[[i32; 3]]) {
    let swung: BTreeSet<BlockPos> = cells.iter().map(|&pos| doors::other_half(world, layer, pos).filter(|other| other[1] < pos[1]).unwrap_or(pos)).collect();
    for pos in swung {
        let Some(name) = level(world, layer).and_then(|level| level.registry().get(level.block(pos[0], pos[1], pos[2]))).map(|state| state.name.to_owned()) else { continue };
        doors::swing(world, None, layer, pos, &name);
    }
}
