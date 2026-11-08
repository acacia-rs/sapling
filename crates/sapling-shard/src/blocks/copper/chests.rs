//! Copper chests, which pair as chests do: a lone chest or a pair's lead half weathers, and a pair
//! weathers, waxes and scrapes as one, each half keeping what it holds (docs/research/copper.md O6,
//! W7; research/captures/bds-p3copper.txt).

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::Copper;
use crate::blocks::{level, send};

/// The cells of the copper chest at `pos`: itself first, then its other half where it is paired.
pub(crate) fn cells(world: &World, layer: Entity, pos: BlockPos) -> Vec<BlockPos> {
    let mut halves = crate::containers::chest_halves(world, layer, pos);
    halves.sort_by_key(|half| *half != pos);
    halves
}

/// Whether a random tick of the copper chest at `pos` may weather it: a pair's other half never
/// does, its lead does for both (O6).
pub(crate) fn ticks(level: &sapling_world::World, pos: BlockPos) -> bool {
    crate::containers::chest_halves_in(level, pos).first() == Some(&pos)
}

/// Makes the copper chest at `pos` and its other half `into`, each keeping its entity. A pair is
/// shown in one UpdateSubchunkBlocks and then both entities, `pos` first; a lone chest by an
/// UpdateBlock, its entity after only where `shows_entity` (BDS sent it after a random tick, not
/// after a wax). Whether anything changed.
pub(crate) fn turn(world: &mut World, layer: Entity, pos: BlockPos, into: Copper, shows_entity: bool) -> bool {
    let cells = cells(world, layer, pos);
    let mut changed = false;
    for &at in &cells {
        let Some(level) = level(world, layer) else { return false };
        let kept = level.block_entity(at).map(|entity| entity.nbt.clone());
        let Some(id) = into.state(level.registry(), level.block(at[0], at[1], at[2])) else { continue };
        changed |= send::set_quietly(world, layer, at, id);
        if let Some(kept) = kept {
            send::keep_entity_quietly(world, layer, at, &kept);
        }
    }
    if !changed {
        return false;
    }
    match cells.as_slice() {
        [_, _] => send::show_together(world, layer, &cells.iter().map(|at| (*at, send::FLAGS)).collect::<Vec<_>>()),
        _ => send::show(world, layer, pos),
    }
    if cells.len() == 2 || shows_entity {
        let Some(level) = level(world, layer) else { return true };
        let shown: Vec<_> = cells.iter().filter_map(|at| level.block_entity(*at).map(|entity| send::block_entity_data(*at, entity))).collect();
        send::to_viewers(world, layer, pos, &shown);
    }
    true
}
