//! A bed as its player's spawn: set by a click on it, risen by after a death, forgotten when it
//! goes (research/captures/bds-bedspawn.txt).

use acacia_proto::packets::{SetSpawnPosition, SetSpawnPositionSpawnType};
use acacia_proto::types::BlockCoordinates;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::{BED, tell};
use crate::components::{InLayer, Layer, Outgoing, Position};

/// The bed a player last clicked, its spawn while it stays: the bed's head, the cell the player
/// stood in, where it rises, and their world.
#[derive(Component, PartialEq)]
pub(crate) struct BedSpawn {
    head: BlockPos,
    spot: BlockPos,
    layer: Entity,
}

impl BedSpawn {
    /// The bed's head and the cell beside it, if they are in `layer`: what storage keeps.
    pub(crate) fn kept(&self, layer: Entity) -> Option<[BlockPos; 2]> {
        (self.layer == layer).then_some([self.head, self.spot])
    }

    pub(crate) fn restored([head, spot]: [BlockPos; 2], layer: Entity) -> Self {
        Self { head, spot, layer }
    }
}

/// Where a dead player rises by its bed (the feet, mid-cell): `None` with no bed of its own in
/// its world, `Some(None)` when that bed is gone, which it is told and which is then forgotten.
pub(crate) fn spawn_of(world: &mut World, player: Entity, layer: Entity) -> Option<Option<[f32; 3]>> {
    let (head, spot) = world.get::<BedSpawn>(player).filter(|bed| bed.layer == layer).map(|bed| (bed.head, bed.spot))?;
    let is_there = world.get::<Layer>(layer).is_some_and(|layer| layer.world.registry().get(layer.world.block(head[0], head[1], head[2])).is_some_and(|state| state.name == BED));
    if !is_there {
        world.entity_mut(player).remove::<BedSpawn>();
        let packet = crate::chat::json_packet(vec![serde_json::json!({ "translate": "tile.bed.notValid" })]);
        if let Some(mut out) = world.get_mut::<Outgoing>(player) {
            out.0.push(packet);
        }
        return Some(None);
    }
    Some(Some([spot[0] as f32 + 0.5, spot[1] as f32, spot[2] as f32 + 0.5]))
}

fn coordinates([x, y, z]: BlockPos) -> BlockCoordinates {
    BlockCoordinates { x, y, z }
}

/// Makes the bed at `head` a player's spawn, telling it when that is news.
pub(super) fn set_spawn(world: &mut World, player: Entity, head: BlockPos) {
    let Some(feet) = world.get::<Position>(player).map(|position| position.feet.map(|c| c.floor() as i32)) else { return };
    let Some(layer) = world.get::<InLayer>(player).map(|layer| layer.0) else { return };
    let is_new = world.get::<BedSpawn>(player).is_none_or(|bed| (bed.head, bed.layer) != (head, layer));
    let packet = SetSpawnPosition { spawn_type: SetSpawnPositionSpawnType::Player, player_position: coordinates(feet), dimension: 0, world_position: coordinates(head) };
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.push(&packet);
    }
    world.entity_mut(player).insert(BedSpawn { head, spot: feet, layer });
    if let Some(mut inventory) = world.get_mut::<crate::inventory::Inventory>(player) {
        inventory.unsaved = true;
    }
    if is_new {
        tell(world, player, "respawnSet");
    }
}
