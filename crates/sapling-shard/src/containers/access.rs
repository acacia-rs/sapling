//! Who may use a container: a chest under a block does not open, and a screen closes once its
//! block is gone (docs/research/beds-and-containers.md, sections 5 and 6).
// TODO: sitting cats are not looked for; shulker boxes are not checked.

use acacia_proto::packets::ContainerClose;
use acacia_proto::types::{WindowID, WindowType};
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::screens::{Viewing, still_shows};
use super::{close, pairs};
use crate::components::{Layer, Outgoing};

/// What a lid needs above a chest, within the cell above (BDS `getObstructionAABB`).
const LID_MIN: [f32; 3] = [1.0 / 16.0, 0.0, 1.0 / 16.0];
const LID_MAX: [f32; 3] = [15.0 / 16.0, 0.5, 15.0 / 16.0];
const LIDDED: &[&str] = &["minecraft:chest", "minecraft:trapped_chest", "minecraft:ender_chest"];

/// Whether something in the cell above `[x, y, z]` collides with the space its lid needs.
fn lid_is_held_down(world: &World, layer: Entity, [x, y, z]: BlockPos) -> bool {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world).filter(|level| level.in_height(y + 1)) else { return false };
    let Some(above) = level.registry().get(level.block(x, y + 1, z)) else { return false };
    let overlaps = |min: [f32; 3], max: [f32; 3]| (0..3).all(|axis| min[axis] < LID_MAX[axis] && max[axis] > LID_MIN[axis]);
    above.boxes.iter().any(|shape| overlaps(shape.min, shape.max))
}

/// Whether the chest `block` at `pos` cannot open: something above either half holds its lid
/// down. Other containers always open.
pub(super) fn is_blocked(world: &World, layer: Entity, pos: BlockPos, block: &str) -> bool {
    LIDDED.contains(&block) && pairs::halves(world, layer, pos).into_iter().any(|half| lid_is_held_down(world, layer, half))
}

/// Whether the block `viewing` shows is still there.
fn is_there(world: &World, viewing: &Viewing) -> bool {
    let [x, y, z] = viewing.pos;
    let Some(level) = world.get::<Layer>(viewing.layer).map(|layer| &layer.world).filter(|level| level.in_height(y)) else { return false };
    level.registry().get(level.block(x, y, z)).is_some_and(|state| still_shows(viewing.screen, state.name))
}

/// Shuts the screen of every player whose block is gone, as BDS does; however far a player walks
/// it stays open (research/captures/bds-reach-close.txt).
pub(crate) fn close_where_gone(world: &mut World) {
    let mut viewers = world.query::<(Entity, &Viewing)>();
    let gone: Vec<(Entity, u8)> = viewers.iter(world).filter(|(_, viewing)| !is_there(world, viewing)).map(|(player, viewing)| (player, viewing.window)).collect();
    for (player, window) in gone {
        close(world, player);
        if let Some(mut out) = world.get_mut::<Outgoing>(player) {
            out.push(&ContainerClose { window_id: WindowID::from_raw(window.into()), window_type: WindowType::Container, server: true });
        }
    }
}
