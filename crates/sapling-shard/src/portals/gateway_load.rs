//! What an End gateway's block actor does once its column is loaded (load 1487fc680, then its first
//! tick 1487fcae0; research/captures/bds-d2gate2.txt): a stored one is shown in that tick; one with no
//! exit, or (0, 0, 0), has a far end opened for it; one with an exit is relinked
//! (dragon/relinks.rs), its exit walked for anew first when `EndGatewayBadPosChecked` is unset, the
//! exit within 100 and the gateway within 300 of the origin (sums of the axes).

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::{ChunkPos, END};

use crate::components::Layer;

const ENTITY: &str = "EndGateway";
const EXIT_NEAR: i32 = 100;
const GATEWAY_NEAR: i32 = 300;

/// Gateways whose column came in and that have not ticked since: the layer, the cell, and whether
/// it came from storage (a generated one, `Age` 0, is only shown).
#[derive(Resource, Default)]
pub(crate) struct Loaded(Vec<(Entity, BlockPos, bool)>);

fn is_gateway(nbt: &Value) -> bool {
    matches!(nbt.get("id"), Some(Value::String(id)) if id.as_str() == ENTITY)
}

/// Notes the gateways of a column of an End that came in.
pub(crate) fn arrived(world: &mut World, layer: Entity, home: ChunkPos) {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world).filter(|level| level.dimension() == END) else { return };
    // Gotcha: a generated gateway was never loaded on BDS, so it keeps its exit; only `Age` tells it from a stored one.
    let stored = |nbt: &Value| !matches!(nbt.get("Age"), None | Some(Value::Int(0)));
    let found: Vec<(Entity, BlockPos, bool)> = level.block_entities_in(home).into_iter().filter(|entity| is_gateway(&entity.nbt)).map(|entity| (layer, entity.pos, stored(&entity.nbt))).collect();
    if !found.is_empty() {
        world.get_resource_or_insert_with(Loaded::default).0.extend(found);
    }
}

/// Forgets gateways whose column went before they ticked.
pub(super) fn forget_gone(world: &mut World) {
    let Some(mut loaded) = world.remove_resource::<Loaded>() else { return };
    loaded.0.retain(|(layer, pos, _)| world.get::<Layer>(*layer).is_some_and(|level| level.world.is_loaded(ChunkPos::of_block(pos[0], pos[2]))));
    world.insert_resource(loaded);
}

/// The first tick of the gateway at `pos` since its column came in, if this is it: opens a far end
/// for a stored one or relinks it; true when it is to be shown (a generated one is, at Age 1).
pub(super) fn first_tick(world: &mut World, layer: Entity, pos: BlockPos, nbt: &Value) -> bool {
    let Some(mut loaded) = world.get_resource_mut::<Loaded>() else { return false };
    let Some(index) = loaded.0.iter().position(|(l, p, _)| *l == layer && *p == pos) else { return false };
    let (.., stored) = loaded.0.swap_remove(index);
    drop(loaded);
    if !stored {
        return false;
    }
    match crate::dragon::gateway_exit(nbt) {
        None | Some([0, 0, 0]) => crate::dragon::open_far_gateway(world, layer, pos),
        Some(exit) => {
            let checked = matches!(nbt.get("EndGatewayBadPosChecked"), Some(Value::Byte(1..)));
            let sum = |at: BlockPos| at.iter().map(|c| c.abs()).sum::<i32>();
            crate::dragon::relink_gateway(world, layer, pos, exit, !checked && sum(exit) < EXIT_NEAR && sum(pos) < GATEWAY_NEAR);
        }
    }
    true
}
