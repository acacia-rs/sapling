//! Who may use a container: a chest under a block does not open, and a screen closes once its
//! block is gone (docs/research/beds-and-containers.md, sections 5 and 6).
// TODO: sitting cats are not looked for.

use acacia_proto::nbt::Value;
use acacia_proto::packets::ContainerClose;
use acacia_proto::types::{WindowID, WindowType};
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::screens::{Screen, Viewing, still_shows};
use super::{close, pairs};
use crate::components::{Layer, Outgoing};

/// What a lid needs above a chest, within the cell above (BDS `getObstructionAABB`).
const LID_MIN: [f32; 3] = [1.0 / 16.0, 0.0, 1.0 / 16.0];
const LID_MAX: [f32; 3] = [15.0 / 16.0, 0.5, 15.0 / 16.0];
const LIDDED: &[&str] = &["minecraft:chest", "minecraft:trapped_chest", "minecraft:ender_chest"];

/// Whether something in the cell `offset` from `[x, y, z]` collides with the space `lid_min..lid_max`
/// in that cell.
fn lid_is_held_down(world: &World, layer: Entity, [x, y, z]: BlockPos, offset: BlockPos, (lid_min, lid_max): ([f32; 3], [f32; 3])) -> bool {
    let [x, y, z] = [x + offset[0], y + offset[1], z + offset[2]];
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world).filter(|level| level.in_height(y)) else { return false };
    let Some(next) = level.registry().get(level.block(x, y, z)) else { return false };
    let overlaps = |min: [f32; 3], max: [f32; 3]| (0..3).all(|axis| min[axis] < lid_max[axis] && max[axis] > lid_min[axis]);
    next.boxes.iter().any(|shape| overlaps(shape.min, shape.max))
}

/// The cell a shulker box's lid opens into, by its entity's `facing` (Bedrock order, up when
/// unset); the lid needs that cell's bottom half whichever way (beds-and-containers.md, section 6).
fn shulker_lid(world: &World, layer: Entity, pos: BlockPos) -> BlockPos {
    let facing = world.get::<Layer>(layer).and_then(|layer| layer.world.block_entity(pos)).and_then(|entity| match entity.nbt.get("facing") {
        Some(Value::Byte(facing)) => Some(*facing),
        _ => None,
    });
    match facing.unwrap_or(1) {
        0 => [0, -1, 0],
        2 => [0, 0, -1],
        3 => [0, 0, 1],
        4 => [-1, 0, 0],
        5 => [1, 0, 0],
        _ => [0, 1, 0],
    }
}

/// Whether the container `block` at `pos` cannot open: something above either half of a chest
/// holds its lid down, or something in the cell a shulker box opens into (a carpet too,
/// research/captures/bds-e2lids.txt). Other containers always open.
pub(super) fn is_blocked(world: &World, layer: Entity, pos: BlockPos, block: &str) -> bool {
    if super::is_shulker_box(block) {
        return lid_is_held_down(world, layer, pos, shulker_lid(world, layer, pos), ([0.0; 3], [1.0, 0.5, 1.0]));
    }
    LIDDED.contains(&block) && pairs::halves(world, layer, pos).into_iter().any(|half| lid_is_held_down(world, layer, half, [0, 1, 0], (LID_MIN, LID_MAX)))
}

/// Whether the block `viewing` shows is still there.
fn is_there(world: &World, viewing: &Viewing) -> bool {
    if let Screen::Carried(vehicle) = viewing.screen {
        return crate::vehicles::cargo(world, vehicle).is_some();
    }
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
