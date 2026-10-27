//! A player's own spawn point: a bed, set by a click on it, a charged respawn anchor in the Nether,
//! set by a click on it, or a cell `/spawnpoint` set; risen by after a death, a bed forgotten when
//! it goes and an anchor when it goes or is spent (research/captures/bds-bedspawn.txt,
//! bds-anchor.txt).

use acacia_proto::packets::{SetSpawnPosition, SetSpawnPositionSpawnType};
use acacia_proto::types::{BlockCoordinates, Vec3f};
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::{BED, tell};
use crate::blocks::{noise, set as set_block, to_viewers, with_property};
use crate::components::{InLayer, Layer, Outgoing, Position};

const ANCHOR: &str = "minecraft:respawn_anchor";
const CHARGE: &str = "respawn_anchor_charge";
const NETHER: i32 = 1;
/// What BDS tells of a spawn point it forgot: every coordinate the least, in no dimension.
const NONE: BlockCoordinates = BlockCoordinates { x: i32::MIN, y: i32::MIN, z: i32::MIN };
const NO_DIMENSION: i32 = 3;

/// A player's own spawn point while it stays: the bed's head or the anchor it last clicked
/// (`None` for one `/spawnpoint` set, which needs no block), the cell it rises in, and their world.
/// A block in the Nether is an anchor: a bed there blows up.
#[derive(Component, PartialEq)]
pub(crate) struct BedSpawn {
    head: Option<BlockPos>,
    pub(crate) spot: BlockPos,
    pub(crate) layer: Entity,
}

impl BedSpawn {
    pub(crate) fn set_by_command(spot: BlockPos, layer: Entity) -> Self {
        Self { head: None, spot, layer }
    }

    /// The block's place and the cell by it, if they are in one of `layers`, and the dimension
    /// they are in: what storage keeps. A spawn point without a block is kept as its cell twice.
    pub(crate) fn kept(&self, world: &World, layers: &[Entity]) -> Option<([BlockPos; 2], i32)> {
        let dimension = world.get::<Layer>(self.layer)?.world.dimension();
        layers.contains(&self.layer).then_some(([self.head.unwrap_or(self.spot), self.spot], dimension))
    }

    pub(crate) fn restored([head, spot]: [BlockPos; 2], layer: Entity) -> Self {
        Self { head: Some(head).filter(|head| *head != spot), spot, layer }
    }
}

/// Where a dead player rises by its own spawn point.
pub(crate) enum Own {
    /// It has none in the realm it died in.
    None,
    /// Its bed is gone (`false`) or its anchor gone or spent (`true`): forgotten, and told once it
    /// has risen ([`tell_lapsed`]).
    Lapsed(bool),
    /// At `feet`, mid-cell, in `layer`, spending a charge of the anchor at `anchor`.
    At { layer: Entity, feet: [f32; 3], anchor: Option<BlockPos> },
}

fn is_anchor(world: &World, layer: Entity) -> bool {
    world.get::<Layer>(layer).is_some_and(|layer| layer.world.dimension() == NETHER)
}

/// The block at `pos` if it is `name`, with its anchor charge.
fn block(world: &World, layer: Entity, [x, y, z]: BlockPos, name: &str) -> Option<u8> {
    let level = &world.get::<Layer>(layer)?.world;
    let state = level.registry().get(level.block(x, y, z)).filter(|state| state.name == name)?;
    Some(state.property(CHARGE).and_then(|charge| charge.parse().ok()).unwrap_or(0))
}

/// Where a dead player rises by its spawn point in one of `realm`'s layers. A bed that is gone
/// is told by its cell first, a spent anchor by a forgotten spawn position, as BDS told them.
pub(crate) fn spawn_of(world: &mut World, player: Entity, realm: &[Entity]) -> Own {
    let Some((head, spot, layer)) = world.get::<BedSpawn>(player).filter(|own| realm.contains(&own.layer)).map(|own| (own.head, own.spot, own.layer)) else { return Own::None };
    let anchor = head.filter(|_| is_anchor(world, layer));
    let stands = match (head, anchor) {
        (None, _) => true,
        (Some(head), None) => block(world, layer, head, BED).is_some(),
        (Some(head), Some(_)) => block(world, layer, head, ANCHOR).is_some_and(|charge| charge > 0),
    };
    if stands {
        return Own::At { layer, feet: [spot[0] as f32 + 0.5, spot[1] as f32, spot[2] as f32 + 0.5], anchor };
    }
    world.entity_mut(player).remove::<BedSpawn>();
    let Some(mut out) = world.get_mut::<Outgoing>(player) else { return Own::Lapsed(anchor.is_some()) };
    match anchor {
        Some(_) => out.push(&SetSpawnPosition { spawn_type: SetSpawnPositionSpawnType::Player, player_position: NONE, dimension: NO_DIMENSION, world_position: NONE }),
        None => out.push(&acacia_proto::packets::Respawn { position: Vec3f { x: spot[0] as f32, y: spot[1] as f32, z: spot[2] as f32 }, state: 0, runtime_entity_id: 0 }),
    }
    Own::Lapsed(anchor.is_some())
}

/// Tells a risen player its bed or anchor was not there: not grey.
pub(crate) fn tell_lapsed(world: &mut World, player: Entity, anchor: bool) {
    let key = if anchor { "tile.respawn_anchor.notValid" } else { "tile.bed.notValid" };
    let packet = crate::chat::json_packet(vec![serde_json::json!({ "translate": key })]);
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.0.push(packet);
    }
}

/// The anchor at `pos` a player rises by is heard spent; [`spend_anchor`] takes the charge.
pub(crate) fn sound_anchor_spent(world: &mut World, layer: Entity, pos: BlockPos) {
    to_viewers(world, layer, pos, &[noise("respawn_anchor.deplete", centre(pos), -1)]);
}

/// The anchor at `pos` a player rose by loses a charge.
pub(crate) fn spend_anchor(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(charge) = block(world, layer, pos, ANCHOR) else { return };
    let Some(level) = world.get::<Layer>(layer) else { return };
    let [x, y, z] = pos;
    if let Some(spent) = with_property(level.world.registry(), level.world.block(x, y, z), CHARGE, &(charge - 1).to_string()) {
        set_block(world, layer, pos, spent);
    }
}

fn centre(pos: BlockPos) -> Vec3f {
    Vec3f { x: pos[0] as f32 + 0.5, y: pos[1] as f32 + 0.5, z: pos[2] as f32 + 0.5 }
}

fn coordinates([x, y, z]: BlockPos) -> BlockCoordinates {
    BlockCoordinates { x, y, z }
}

/// Makes the bed at `head` a player's spawn, telling it when that is news.
pub(super) fn set_spawn(world: &mut World, player: Entity, head: BlockPos) {
    if let Some(is_new) = make_spawn(world, player, head) {
        if is_new {
            tell(world, player, "tile.bed.respawnSet");
        }
    }
}

/// A click on a charged anchor in the Nether makes it the player's spawn, told and heard; false,
/// with nothing told, where it already was (research/captures/bds-anchor.txt).
pub(crate) fn set_anchor_spawn(world: &mut World, player: Entity, layer: Entity, pos: BlockPos) -> bool {
    if !is_anchor(world, layer) || world.get::<BedSpawn>(player).is_some_and(|own| (own.head, own.layer) == (Some(pos), layer)) {
        return false;
    }
    if make_spawn(world, player, pos).is_none() {
        return false;
    }
    tell(world, player, "tile.respawn_anchor.respawnSet");
    to_viewers(world, layer, pos, &[noise("respawn_anchor.set_spawn", centre(pos), -1)]);
    true
}

/// Makes the block at `head` a player's spawn, from the cell it stands in, and tells its client;
/// whether that is news. `None` for a player not in a world.
fn make_spawn(world: &mut World, player: Entity, head: BlockPos) -> Option<bool> {
    let feet = world.get::<Position>(player)?.feet.map(|c| c.floor() as i32);
    let layer = world.get::<InLayer>(player)?.0;
    let dimension = world.get::<Layer>(layer)?.world.dimension();
    let is_new = world.get::<BedSpawn>(player).is_none_or(|bed| (bed.head, bed.layer) != (Some(head), layer));
    let packet = SetSpawnPosition { spawn_type: SetSpawnPositionSpawnType::Player, player_position: coordinates(feet), dimension, world_position: coordinates(head) };
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.push(&packet);
    }
    world.entity_mut(player).insert(BedSpawn { head: Some(head), spot: feet, layer });
    if let Some(mut inventory) = world.get_mut::<crate::inventory::Inventory>(player) {
        inventory.unsaved = true;
    }
    Some(is_new)
}
