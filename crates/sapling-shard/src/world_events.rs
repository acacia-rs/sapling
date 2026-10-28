//! What the world does on its own, and portal trips, offered to plugins (sapling_api's
//! block_event.rs and PortalUseEvent), each asked with one line from its mechanic's module.

use bevy_ecs::prelude::*;
use sapling_api::{Block, BlockPos, CropTrampleEvent, LeavesDecayEvent, LiquidHardenEvent, Portal, PortalCreateEvent, PortalUseEvent, RedstoneUpdateEvent};

use crate::components::in_world_identity;
use crate::{plugins, worlds};

pub(crate) fn may_harden(world: &mut World, layer: Entity, pos: BlockPos, into: &str) -> bool {
    !plugins::stopped_in(world, layer, |_| LiquidHardenEvent::new(pos, Block::new(into)))
}

pub(crate) fn may_trample(world: &mut World, layer: Entity, farmland: BlockPos) -> bool {
    !plugins::stopped_in(world, layer, |_| CropTrampleEvent::new(farmland))
}

/// Whether plugins let the block at `pos` change at its random tick: asked only of leaves, the
/// one random tick of theirs that does anything being their decay.
pub(crate) fn may_decay(world: &mut World, layer: Entity, pos: BlockPos) -> bool {
    if !plugins::listening::<LeavesDecayEvent>(world) {
        return true;
    }
    let Some(leaves) = crate::blocks::block_at(world, layer, pos).filter(|block| block.name.ends_with("leaves")) else { return true };
    !plugins::offer_in(world, layer, LeavesDecayEvent::new(pos, leaves)).is_cancelled()
}

pub(crate) fn may_create_portal(world: &mut World, layer: Entity, cells: &[(BlockPos, u32)]) -> bool {
    !plugins::stopped_in(world, layer, |_| PortalCreateEvent::new(cells.iter().map(|(pos, _)| *pos).collect()))
}

pub(crate) fn redstone_updated(world: &mut World, layer: Entity, pos: BlockPos) {
    if plugins::listening::<RedstoneUpdateEvent>(world) {
        plugins::offer_in(world, layer, RedstoneUpdateEvent::new(pos));
    }
}

pub(crate) fn may_use_portal(world: &mut World, player: Entity, portal: Portal) -> bool {
    let Some(who) = in_world_identity(world, player).filter(|_| plugins::listening::<PortalUseEvent>(world)) else { return true };
    let before = worlds::focused(world);
    worlds::focus_on(world, player);
    let cancelled = plugins::offer(world, PortalUseEvent::new(who, portal)).is_cancelled();
    worlds::focus(world, before);
    !cancelled
}
