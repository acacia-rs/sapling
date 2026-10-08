//! Breaking, placing and clicking blocks: what a client asks for is checked, offered to the
//! handlers and then either done for everyone who sees the place or undone on the client that
//! predicted it (docs/DESIGN.md "Step 6 as built"; request.rs). Blocks that change on their own:
//! falling, liquids, plates, updates.

mod buckets;
mod doors;
mod entities;
mod facing;
mod falling;
mod ground;
mod items;
mod liquid;
mod loot;
mod mining;
mod place;
mod plates;
mod pots;
mod rails;
mod redstone;
mod request;
mod send;
mod shapes;
mod sign_ink;
mod signs;
mod updates;
mod waterlog;

pub(crate) use doors::{bed_head, release_buttons, with as with_property};
pub(crate) use plates::press_plates;
pub(crate) use redstone::{apply as apply_redstone, evaluate as evaluate_redstone, power_at};
pub(crate) use signs::dispatch_edits as dispatch_sign_edits;
pub(crate) use loot::Dice;
pub(crate) use liquid::tick as flow_liquids;
pub(crate) use mining::Breaking;
pub(crate) use request::{BlockInputs, dispatch};
use request::{Request, Site, site};
pub(crate) use send::{block_at, keep_entity, put_block, set, sound, to_viewers};
pub(crate) use updates::break_block;

use bevy_ecs::prelude::*;
use sapling_api::{Block, BlockPos, GameMode};
use sapling_data::EYE_HEIGHT;

use crate::components::{Layer, Mode, Position};

/// Blocks from the eye to the middle of a block: BDS took a click at 6.4 and 12.2 (creative) and
/// refused 7.3 and 13.2.
const REACH: f32 = 7.0;
const CREATIVE_REACH: f32 = 13.0;

fn level(world: &World, layer: Entity) -> Option<&sapling_world::World> {
    world.get::<Layer>(layer).map(|layer| &layer.world)
}

/// Whether the middle of the block at `pos` is within the player's reach of its eyes.
pub(crate) fn reaches(world: &World, player: Entity, pos: BlockPos) -> bool {
    let (Some(position), Some(mode)) = (world.get::<Position>(player), world.get::<Mode>(player)) else { return false };
    let eye = [position.feet[0], position.feet[1] + EYE_HEIGHT, position.feet[2]];
    let distance: f32 = (0..3).map(|axis| (pos[axis] as f32 + 0.5 - eye[axis]).powi(2)).sum();
    let reach = if mode.0 == GameMode::Creative { CREATIVE_REACH } else { REACH };
    distance <= reach * reach
}

/// Sets a block for a handler, as a player's place puts it (concrete powder by water as concrete),
/// and settles what is around it; a cell that already held it changes nothing. False where it
/// could not be set.
pub(crate) fn update_block(world: &mut World, layer: Entity, pos: BlockPos, block: &Block) -> bool {
    let registry = level(world, layer).map(|level| level.registry());
    let put_as = registry.and_then(|registry| send::resolve(registry, block)).map(|id| (falling::as_put(world, layer, pos, id), id));
    let swapped = match (registry, put_as) {
        (Some(registry), Some((put, id))) if put != id => Some(send::describe(registry, put)),
        _ => None,
    };
    let put = put_block(world, layer, pos, swapped.as_ref().unwrap_or(block));
    if put == Some(true) {
        updates::settle(world, layer, pos);
    }
    put.is_some()
}

/// Moves falling blocks and starts the falls that are due, then settles what they changed.
pub(crate) fn fall_blocks(world: &mut World) {
    for (layer, pos) in falling::tick(world) {
        updates::settle(world, layer, pos);
    }
}
