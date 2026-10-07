//! What those near an open container see and hear: a chest's lid (`BlockEvent` ChangeState, 1
//! open and 0 shut, as BDS's `ChestBlockActor::startOpen` raises it), a barrel's `open_bit`, and
//! their sounds.
// TODO: whether barrels, shulker boxes and ender chests sound late too, as chests do, is unrecorded.

use acacia_proto::packets::{BlockEvent, BlockEventType};
use acacia_proto::types::BlockCoordinates;
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_api::BlockPos;

use crate::blocks;
use crate::components::{Layer, Tick};

/// Ticks from a chest's opening and shutting to its sound, as BDS sent them
/// (research/captures/containers-findings.md D5): the lid swings first.
const CHEST_LATE: [u64; 2] = [1, 5];

/// Sounds due later: when, the world, where, the sound.
#[derive(Resource, Default)]
pub(crate) struct LateSounds(Vec<(u64, Entity, BlockPos, Bytes)>);

/// Plays the sounds whose time has come.
pub(crate) fn play_late_sounds(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let Some(mut late) = world.get_resource_mut::<LateSounds>() else { return };
    let (due, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut late.0).into_iter().partition(|(when, ..)| *when <= now);
    late.0 = waiting;
    for (_, layer, pos, sound) in due {
        blocks::to_viewers(world, layer, pos, &[sound]);
    }
}

/// Ticks between two showings of an open lid (BDS `ChestBlockActor::tick`).
pub(super) const REPEAT: u64 = 80;
const BARREL: &str = "minecraft:barrel";

fn event(world: &mut World, layer: Entity, halves: &[BlockPos], open: bool) {
    let has_lid = |pos: &&BlockPos| super::block_name(world, layer, **pos).and_then(super::kind).is_some_and(|kind| kind.has_lid);
    let lidded: Vec<BlockPos> = halves.iter().filter(has_lid).copied().collect();
    for [x, y, z] in lidded {
        let lid = BlockEvent { position: BlockCoordinates { x, y, z }, r#type: BlockEventType::ChangeState, data: open.into() };
        blocks::to_viewers(world, layer, [x, y, z], &[sapling_net::encode(&lid)]);
    }
}

/// Shows the lid of a container that is open once more.
pub(super) fn show_open(world: &mut World, layer: Entity, halves: &[BlockPos]) {
    event(world, layer, halves, true);
}

/// The sounds a container makes opening and shutting, by its block (names from BDS's binary and
/// what it sent), and how many ticks after each.
fn sounds(name: &str) -> Option<([&'static str; 2], [u64; 2])> {
    match name {
        BARREL => Some((["block.barrel.open", "block.barrel.close"], [0; 2])),
        "minecraft:ender_chest" => Some((["enderchest.open", "enderchest.closed"], [0; 2])),
        "minecraft:chest" | "minecraft:trapped_chest" => Some((["chest.open", "chest.closed"], CHEST_LATE)),
        _ if super::is_shulker_box(name) => Some((["shulkerbox.open", "shulkerbox.closed"], [0; 2])),
        _ => None,
    }
}

/// Opens or shuts what those near the container `halves` see and hear of it: the lid, a
/// barrel's top.
pub(super) fn set(world: &mut World, layer: Entity, halves: &[BlockPos], open: bool) {
    event(world, layer, halves, open);
    let Some(&pos) = halves.first() else { return };
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let (registry, id) = (level.registry(), level.block(pos[0], pos[1], pos[2]));
    let Some(name) = registry.get(id).map(|state| state.name) else { return };
    let Some(([opening, shutting], late)) = sounds(name) else { return };
    let heard = blocks::sound(if open { opening } else { shutting }, registry, pos, id);
    let turned = blocks::with_property(registry, id, "open_bit", if open { "1" } else { "0" }).filter(|_| name == BARREL);
    if let Some(turned) = turned {
        blocks::set(world, layer, pos, turned);
    }
    match late[usize::from(!open)] {
        0 => blocks::to_viewers(world, layer, pos, &[heard]),
        after => {
            let due = world.resource::<Tick>().0 + after;
            world.get_resource_or_insert_with(LateSounds::default).0.push((due, layer, pos, heard));
        }
    }
}
