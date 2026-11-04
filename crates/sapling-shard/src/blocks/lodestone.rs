//! Lodestones: a compass or a lodestone compass clicked on one becomes a lodestone compass whose
//! `trackingHandle` names the lodestone's target in the realm's `PositionTrackingDB` (one target
//! a lodestone, made at the first binding), and a client asks for a handle's target by
//! `PositionTrackingDBClientRequest` (docs/research/blocks.md). BDS tells those who see it a
//! target's record unasked when a binding makes it and when its lodestone goes; a second
//! binding to the same lodestone told nothing (research/captures/bds-t3uses.txt).

use acacia_proto::nbt::{Nbt, Value};
use acacia_proto::packets::{PositionTrackingDbBroadcast, PositionTrackingDbBroadcastBroadcastAction as Told, PositionTrackingDbRequest};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::{ItemStack, TrackerRecord};

use super::{level, send};
use crate::components::{InLayer, Layer, Outgoing, PacketQueues};

const LODESTONE: &str = "minecraft:lodestone";
const COMPASSES: [&str; 2] = ["minecraft:compass", "minecraft:lodestone_compass"];
const BOUND: &str = "minecraft:lodestone_compass";
const HANDLE: &str = "trackingHandle";
/// A target whose lodestone is gone, as BDS's `Destory` record said it.
const GONE: u8 = 2;

fn is_lodestone(world: &World, layer: Entity, pos: BlockPos) -> Option<bool> {
    let level = level(world, layer).filter(|level| level.in_height(pos[1]) && level.is_loaded(send::column(pos)))?;
    Some(level.registry().get(level.block(pos[0], pos[1], pos[2])).is_some_and(|state| state.name == LODESTONE))
}

fn broadcast(target: &TrackerRecord) -> PositionTrackingDbBroadcast {
    let told = if target.status == GONE { Told::Destory } else { Told::Update };
    PositionTrackingDbBroadcast { broadcast_action: told, tracking_id: target.handle as i32, nbt: Nbt { name: String::new(), value: target.compound() }.into() }
}

/// The target at `pos` in `layer`, made if there is none, and whether it was made.
fn target(world: &mut World, layer: Entity, pos: BlockPos) -> Option<(TrackerRecord, bool)> {
    let dimension = world.get::<Layer>(layer)?.world.dimension();
    let home = crate::dimensions::home(world, layer);
    let mut home = world.get_mut::<Layer>(home)?;
    let level = home.world.level_mut();
    if let Some(found) = level.trackers.iter_mut().find(|target| target.dimension == dimension && target.pos == pos) {
        found.status = 0;
        return Some((*found, false));
    }
    level.last_tracker += 1;
    let made = TrackerRecord { handle: level.last_tracker, dimension, pos, status: 0 };
    level.trackers.push(made);
    Some((made, true))
}

/// A click with `held` on the block at `pos`: a compass binds to a lodestone there, the one in
/// the hand swapped for a bound one; in creative the bound one is given and the hand kept, as
/// BDS did. False for anything else.
pub(super) fn bind(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, held: &str, creative: bool) -> bool {
    if !COMPASSES.contains(&held) || is_lodestone(world, layer, pos) != Some(true) {
        return false;
    }
    let Some((target, made)) = target(world, layer, pos) else { return false };
    if made {
        send::to_viewers(world, layer, pos, &[sapling_net::encode(&broadcast(&target))]);
    }
    let tag = Value::Compound(vec![(HANDLE.into(), Value::Int(target.handle as i32))]);
    let bound = ItemStack { name: BOUND.into(), count: 1, damage: 0, block: None, tag: Some(tag) };
    if creative {
        crate::inventory::change(world, player, |inventory| inventory.add(&bound));
    } else {
        crate::inventory::swap_held_for(world, player, bound);
    }
    let [x, y, z] = pos.map(|c| c as f32 + 0.5);
    send::to_viewers(world, layer, pos, &[send::noise("lodestone_compass.link_compass_to_lodestone", Vec3f { x, y, z }, -1)]);
    true
}

/// Marks the target at `pos` gone, telling those who see it, when the lodestone there went.
pub(super) fn noticed(world: &mut World, layer: Entity, pos: BlockPos) {
    if is_lodestone(world, layer, pos) != Some(false) {
        return;
    }
    let Some(dimension) = world.get::<Layer>(layer).map(|layer| layer.world.dimension()) else { return };
    let home = crate::dimensions::home(world, layer);
    let Some(mut home) = world.get_mut::<Layer>(home) else { return };
    let Some(target) = home.world.level_mut().trackers.iter_mut().find(|target| target.dimension == dimension && target.pos == pos && target.status != GONE) else { return };
    target.status = GONE;
    let told = broadcast(target);
    send::to_viewers(world, layer, pos, &[sapling_net::encode(&told)]);
}

/// Answers this tick's requests for a handle's target: its record, marked gone once its loaded
/// lodestone is; not found for a handle the realm never gave.
pub(crate) fn answer(world: &mut World) {
    for (player, request) in world.resource_mut::<PacketQueues>().take::<PositionTrackingDbRequest>() {
        let Some(layer) = world.get::<InLayer>(player).map(|layer| layer.0) else { continue };
        let home = crate::dimensions::home(world, layer);
        let handle = request.tracking_id as u32;
        let found = world.get::<Layer>(home).and_then(|home| home.world.level().trackers.iter().find(|target| target.handle == handle).copied());
        let packet = match found {
            Some(mut target) => {
                if let Some(at) = crate::dimensions::of(world, home, target.dimension)
                    && is_lodestone(world, at, target.pos) == Some(false)
                {
                    target.status = GONE;
                    if let Some(mut home) = world.get_mut::<Layer>(home) {
                        home.world.level_mut().trackers.iter_mut().filter(|kept| kept.handle == handle).for_each(|kept| kept.status = GONE);
                    }
                }
                broadcast(&target)
            }
            None => PositionTrackingDbBroadcast { broadcast_action: Told::NotFound, tracking_id: request.tracking_id, nbt: Nbt { name: String::new(), value: Value::Compound(Vec::new()) }.into() },
        };
        if let Some(mut out) = world.get_mut::<Outgoing>(player) {
            out.push(&packet);
        }
    }
}
