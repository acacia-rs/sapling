//! Who may use a container: a chest under a block does not open, and a screen closes once its
//! block is gone (docs/research/beds-and-containers.md, sections 5 and 6).

use acacia_proto::nbt::Value;
use acacia_proto::packets::ContainerClose;
use acacia_proto::types::{WindowID, WindowType};
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::screens::{Screen, Viewing, still_shows};
use super::{close, pairs};
use crate::body::Body;
use crate::components::{InLayer, Layer, Outgoing};
use crate::creatures::Raised;
use crate::entities::Kind;

/// What a lid needs above a chest, within the cell above (BDS `getObstructionAABB`).
const LID_MIN: [f32; 3] = [1.0 / 16.0, 0.0, 1.0 / 16.0];
const LID_MAX: [f32; 3] = [15.0 / 16.0, 0.5, 15.0 / 16.0];
const ENDER_CHEST: &str = "minecraft:ender_chest";
const SITTERS: [&str; 2] = ["minecraft:cat", "minecraft:ocelot"];

/// Whether a block never holds a lid down whatever its shape (`minecraft:chest_obstruction` `never`): a
/// hopper, as BDS opened a chest under one (research/captures/bds-p2hopper.txt), and the pack's concrete
/// and wool stairs.
fn never_obstructs(name: &str) -> bool {
    name == "minecraft:hopper" || name.ends_with("_concrete_stairs") || name.ends_with("_wool_stairs")
}

/// Whether something in the cell `offset` from `[x, y, z]` collides with the space `lid_min..lid_max`
/// in that cell; a chest's lid (`by_rule`) asks the block's obstruction rule first.
fn lid_is_held_down(world: &World, layer: Entity, [x, y, z]: BlockPos, offset: BlockPos, (lid_min, lid_max): ([f32; 3], [f32; 3]), by_rule: bool) -> bool {
    let [x, y, z] = [x + offset[0], y + offset[1], z + offset[2]];
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world).filter(|level| level.in_height(y)) else { return false };
    let Some(next) = level.registry().get(level.block(x, y, z)).filter(|next| !(by_rule && never_obstructs(next.name))) else { return false };
    let overlaps = |min: [f32; 3], max: [f32; 3]| (0..3).all(|axis| min[axis] < lid_max[axis] && max[axis] > lid_min[axis]);
    next.boxes.iter().any(|shape| overlaps(shape.min, shape.max))
}

/// Whether a sitting cat or ocelot is in the space a chest's lid needs (BDS
/// `ChestBlockActor::_detectEntityObstruction` 141f33660).
fn cat_sits_on(world: &mut World, layer: Entity, [x, y, z]: BlockPos) -> bool {
    let [x, y, z] = [x as f32, y as f32 + 1.0, z as f32];
    let (lid_min, lid_max) = ([x + LID_MIN[0], y + LID_MIN[1], z + LID_MIN[2]], [x + LID_MAX[0], y + LID_MAX[1], z + LID_MAX[2]]);
    let mut cats = world.query::<(&Kind, &InLayer, &Body, &Raised)>();
    cats.iter(world).filter(|(kind, on, _, raised)| SITTERS.contains(&kind.0.as_str()) && on.0 == layer && raised.kept.sitting).any(|(.., body, _)| {
        let (min, max) = body.bounds();
        (0..3).all(|axis| min[axis] < lid_max[axis] && max[axis] > lid_min[axis])
    })
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
/// research/captures/bds-e2lids.txt); a cat sitting on a chest holds it shut too. Other containers
/// always open.
pub(super) fn is_blocked(world: &mut World, layer: Entity, pos: BlockPos, block: &str) -> bool {
    if super::is_shulker_box(block) {
        return lid_is_held_down(world, layer, pos, shulker_lid(world, layer, pos), ([0.0; 3], [1.0, 0.5, 1.0]), false);
    }
    (super::is_chest(block) || block == ENDER_CHEST) &&pairs::halves(world, layer, pos).into_iter().any(|half| lid_is_held_down(world, layer, half, [0, 1, 0], (LID_MIN, LID_MAX), true) || cat_sits_on(world, layer, half))
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
