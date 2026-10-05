//! What those near an open container see and hear: a chest's lid (`BlockEvent` ChangeState, 1
//! open and 0 shut, as BDS's `ChestBlockActor::startOpen` raises it), a barrel's `open_bit`.
// TODO: a chest's own open and close sounds: their names in this protocol were not found
// (a barrel's, `block.barrel.open` / `block.barrel.close`, are strings in the server's binary).

use acacia_proto::packets::{BlockEvent, BlockEventType};
use acacia_proto::types::BlockCoordinates;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use crate::blocks;
use crate::components::Layer;

/// Ticks between two showings of an open lid (BDS `ChestBlockActor::tick`).
pub(super) const REPEAT: u64 = 80;
const BARREL: &str = "minecraft:barrel";

fn event(world: &mut World, layer: Entity, halves: &[BlockPos], open: bool) {
    for &[x, y, z] in halves {
        let lid = BlockEvent { position: BlockCoordinates { x, y, z }, r#type: BlockEventType::ChangeState, data: open.into() };
        blocks::to_viewers(world, layer, [x, y, z], &[sapling_net::encode(&lid)]);
    }
}

/// Shows the lid of a container that is open once more.
pub(super) fn show_open(world: &mut World, layer: Entity, halves: &[BlockPos]) {
    event(world, layer, halves, true);
}

/// Opens or shuts what those near the container `halves` see of it: the lid, and a barrel's top.
pub(super) fn set(world: &mut World, layer: Entity, halves: &[BlockPos], open: bool) {
    event(world, layer, halves, open);
    let Some(&pos) = halves.first() else { return };
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let (registry, id) = (level.registry(), level.block(pos[0], pos[1], pos[2]));
    if registry.get(id).is_none_or(|state| state.name != BARREL) {
        return;
    }
    let heard = blocks::sound(if open { "block.barrel.open" } else { "block.barrel.close" }, registry, pos, id);
    let turned = blocks::with_property(registry, id, "open_bit", if open { "1" } else { "0" });
    if let Some(turned) = turned {
        blocks::set(world, layer, pos, turned);
    }
    blocks::to_viewers(world, layer, pos, &[heard]);
}
