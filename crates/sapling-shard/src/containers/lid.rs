//! What those near an open container see and hear: a chest's lid (`BlockEvent` ChangeState, 1
//! open and 0 shut, as BDS's `ChestBlockActor::startOpen` raises it), a barrel's `open_bit`, and
//! their sounds, each as late as BDS sent it (research/captures/bds-shut.txt).

use acacia_proto::packets::{BlockEvent, BlockEventType};
use acacia_proto::types::{BlockCoordinates, Vec3f};
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use crate::components::{Layer, Tick};
use crate::{blocks, late_sounds};

/// Ticks from a chest's opening and shutting to its sound: the lid swings first.
const CHEST_LATE: [u64; 2] = [1, 5];
/// The same of an ender chest and a barrel.
const LATER: [u64; 2] = [2, 6];
/// The same of a shulker box, which is heard shut once its lid is all the way down.
const SHULKER_LATE: [u64; 2] = [2, 11];

/// Ticks between two showings of an open lid (BDS `ChestBlockActor::tick`).
pub(super) const REPEAT: u64 = 80;
const BARREL: &str = "minecraft:barrel";

/// Barrels turned, shown a tick later: when, the world, where.
#[derive(Resource, Default)]
pub(crate) struct TurnedBarrels(Vec<(u64, Entity, BlockPos)>);

/// The lid shown at the lead half only: BDS's double chests, plain and copper, sent no event at the
/// other (research/captures/bds-u3chest.txt).
fn event(world: &mut World, layer: Entity, halves: &[BlockPos], open: bool) {
    let Some(&[x, y, z]) = halves.first() else { return };
    if !super::block_name(world, layer, [x, y, z]).and_then(super::kind).is_some_and(|kind| kind.has_lid) {
        return;
    }
    let lid = BlockEvent { position: BlockCoordinates { x, y, z }, r#type: BlockEventType::ChangeState, data: open.into() };
    blocks::to_viewers(world, layer, [x, y, z], &[sapling_net::encode(&lid)]);
}

/// Where a container is heard: the middle of its halves (BDS's double chest from the seam).
fn middle(halves: &[BlockPos]) -> Vec3f {
    let along = |axis: usize| halves.iter().map(|pos| pos[axis] as f32 + 0.5).sum::<f32>() / halves.len().max(1) as f32;
    Vec3f { x: along(0), y: along(1), z: along(2) }
}

/// Shows the lid of a container that is open once more.
pub(super) fn show_open(world: &mut World, layer: Entity, halves: &[BlockPos]) {
    event(world, layer, halves, true);
}

/// The sounds a container makes opening and shutting, by its block (names from BDS's binary and
/// what it sent), and how many ticks after each.
fn sounds(name: &str) -> Option<([&'static str; 2], [u64; 2])> {
    match name {
        BARREL => Some((["block.barrel.open", "block.barrel.close"], LATER)),
        "minecraft:ender_chest" => Some((["enderchest.open", "enderchest.closed"], LATER)),
        _ if super::is_chest(name) => Some((["chest.open", "chest.closed"], CHEST_LATE)),
        _ if super::is_shulker_box(name) => Some((["shulkerbox.open", "shulkerbox.closed"], SHULKER_LATE)),
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
    let turned = blocks::with_property(registry, id, "open_bit", if open { "1" } else { "0" }).filter(|_| name == BARREL);
    // A barrel's sound names the state it turns to.
    let heard = blocks::sound_at(if open { opening } else { shutting }, registry, middle(halves), turned.unwrap_or(id));
    if turned.is_some_and(|turned| blocks::set_quietly(world, layer, pos, turned)) {
        let due = world.resource::<Tick>().0 + 1;
        world.get_resource_or_insert_with(TurnedBarrels::default).0.push((due, layer, pos));
    }
    late_sounds::play_later(world, late[usize::from(!open)], layer, pos, heard);
}

/// The shutting sound of the container at `from`, heard at `to` once its own wait and `after` more
/// ticks are over, without a lid shown: one a piston carries.
pub(super) fn heard_shut_at(world: &mut World, layer: Entity, from: BlockPos, to: BlockPos, after: u64) {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let (registry, id) = (level.registry(), level.block(from[0], from[1], from[2]));
    let Some(([_, shutting], late)) = registry.get(id).and_then(|state| sounds(state.name)) else { return };
    let heard = blocks::sound(shutting, registry, to, id);
    late_sounds::play_later(world, after + late[1], layer, to, heard);
}

/// Shows the barrels turned a tick ago, with what they hold, as BDS did.
pub(crate) fn show_turned_barrels(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let Some(mut turned) = world.get_resource_mut::<TurnedBarrels>() else { return };
    let (due, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut turned.0).into_iter().partition(|(when, ..)| *when <= now);
    turned.0 = waiting;
    for (_, layer, pos) in due {
        blocks::show_whole(world, layer, pos);
    }
}
