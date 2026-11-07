//! Cells a falling block took or left, as BDS shows them (research/captures/bds-ai1fill.txt, bds-al2fill.txt):
//! a cell alone in an UpdateBlockSynced, the cells of one tick's falls in one sub-chunk together in an
//! UpdateSubchunkBlocks, each naming its entity.

use acacia_proto::packets::{UpdateBlockSynced, UpdateSubchunkBlocks};
use acacia_proto::types::{BlockCoordinates, BlockUpdate, BlockUpdateTransitionType};
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::{FLAGS, Transition, to_viewers};
use crate::components::Layer;

/// A cell, the entity that took or left it, and which it did.
type Synced = (BlockPos, u64, Transition);

/// What [`tell_synced`] was told while [`synced_together`] runs, with each cell's world.
#[derive(Resource, Default)]
struct Kept(Option<Vec<(Entity, Synced)>>);

/// Runs `falls` keeping what [`tell_synced`] is told, then shows it a sub-chunk at a time, sub-chunks in
/// the order first told.
pub(crate) fn synced_together<R>(world: &mut World, falls: impl FnOnce(&mut World) -> R) -> R {
    let opens = world.get_resource::<Kept>().is_none_or(|kept| kept.0.is_none());
    if opens {
        world.get_resource_or_insert_with(Kept::default).0 = Some(Vec::new());
    }
    let result = falls(world);
    if opens {
        show_kept(world);
    }
    result
}

/// Shows everyone who sees `pos` its block as the entity `entity` took it (`Create`: a falling block
/// leaving it) or left it (`Destroy`: one landing), now or as [`synced_together`] closes.
pub(crate) fn tell_synced(world: &mut World, layer: Entity, pos: BlockPos, entity: u64, transition: Transition) {
    match world.get_resource_mut::<Kept>().and_then(|kept| kept.into_inner().0.as_mut()) {
        Some(kept) => kept.push((layer, (pos, entity, transition))),
        None => show(world, layer, &[(pos, entity, transition)]),
    }
}

fn show_kept(world: &mut World) {
    let Some(kept) = world.get_resource_mut::<Kept>().and_then(|mut kept| kept.0.take()) else { return };
    let mut sections: Vec<((Entity, BlockPos), Vec<Synced>)> = Vec::new();
    for (layer, cell) in kept {
        let section = (layer, corner(cell.0));
        match sections.iter_mut().find(|(at, _)| *at == section) {
            Some((_, cells)) => cells.push(cell),
            None => sections.push((section, vec![cell])),
        }
    }
    for ((layer, _), cells) in sections {
        show(world, layer, &cells);
    }
}

fn corner(pos: BlockPos) -> BlockPos {
    pos.map(|c| c.div_euclid(16) * 16)
}

/// `cells` of one sub-chunk, each with its block as it is now.
fn show(world: &mut World, layer: Entity, cells: &[Synced]) {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let hash = |[x, y, z]: BlockPos| sapling_world::block_hash(level.registry(), level.block(x, y, z));
    let at = |[x, y, z]: BlockPos| BlockCoordinates { x, y, z };
    let packet = match cells {
        [] => return,
        &[(pos, entity, transition)] => {
            sapling_net::encode(&UpdateBlockSynced { position: at(pos), block_runtime_id: hash(pos), flags: FLAGS, layer: 0, entity_unique_id: entity, transition_type: transition })
        }
        _ => {
            let listed = |&(pos, entity, transition): &Synced| BlockUpdate {
                position: at(pos),
                runtime_id: hash(pos),
                flags: FLAGS.0,
                entity_unique_id: as_written(entity),
                transition_type: if transition == Transition::Destroy { BlockUpdateTransitionType::Destroy } else { BlockUpdateTransitionType::Create },
            };
            let [x, y, z] = corner(cells[0].0);
            sapling_net::encode(&UpdateSubchunkBlocks { x, y, z, blocks: cells.iter().map(listed).collect(), extra: Vec::new() })
        }
    };
    to_viewers(world, layer, cells[0].0, &[packet]);
}

/// The field value that puts `id`'s own bits on the wire: BDS writes the id unsigned where acacia's
/// field is zigzagged (its ids read back as `id >> 1`, flipped for odd ones; bds-al2fill.txt).
fn as_written(id: u64) -> i64 {
    ((id >> 1) as i64) ^ -((id & 1) as i64)
}
