//! A stuck arrow or a lying trident picked up by a player who walks over it, as an item is
//! (drops.rs reach): a survival player with room is given what it was loosed as, a creative one
//! takes it away and is given nothing; a shot from creative or with Infinity only a creative
//! player takes (`Pickup`).

use acacia_proto::packets::TakeItemEntity;
use bevy_ecs::prelude::*;
use sapling_api::GameMode;

use crate::components::{InLayer, Mode, NetId, Position, in_world};
use crate::creatures::{Arrow, Loosed, Pickup};
use crate::inventory::{self, Inventory};
use crate::{blocks, drops, entities};

/// The box a lying shot is reached by: a lying item's.
const SIZE: f32 = 0.25;

/// Hands each stuck shot to the first player standing on it who may have it.
pub(super) fn tick(world: &mut World) {
    let lying = crate::creatures::lying_shots(world);
    if lying.is_empty() {
        return;
    }
    let players: Vec<(Entity, Entity, [f32; 3], u64, GameMode)> = in_world(world)
        .into_iter()
        .filter_map(|player| Some((player, world.get::<InLayer>(player)?.0, world.get::<Position>(player)?.feet, world.get::<NetId>(player)?.0, world.get::<Mode>(player)?.0)))
        .collect();
    for (shot, layer, at, id, loosed) in lying {
        let near = players.iter().filter(|(_, on, feet, ..)| *on == layer && drops::within_pickup(*feet, at, SIZE));
        let taker = near.copied().find(|(player, _, _, _, mode)| match (mode, loosed.pickup) {
            (GameMode::Creative, _) => true,
            (GameMode::Survival | GameMode::Adventure, Pickup::Anyone) => loosed.item.as_ref().is_some_and(|item| world.get::<Inventory>(*player).is_some_and(|inventory| inventory.loaded && inventory.has_room_for(item))),
            _ => false,
        });
        let Some((player, _, _, player_id, mode)) = taker else { continue };
        take(world, shot, layer, at, id, player, player_id, (mode != GameMode::Creative).then_some(loosed).and_then(|loosed| loosed.item));
    }
}

/// `player` takes the shot, and is given `item` if anything.
#[allow(clippy::too_many_arguments)]
pub(super) fn take(world: &mut World, shot: Entity, layer: Entity, at: [f32; 3], id: u64, player: Entity, player_id: u64, item: Option<sapling_world::ItemStack>) {
    if let Some(item) = item {
        let left = inventory::change(world, player, |inventory| inventory.add(&item)).unwrap_or(item.count);
        if left > 0 {
            return;
        }
    }
    let taken = sapling_net::encode(&TakeItemEntity { runtime_entity_id: id, target: player_id as u32 });
    blocks::to_viewers(world, layer, at.map(|c| c.floor() as i32), &[taken]);
    world.entity_mut(shot).remove::<(Arrow, Loosed)>();
    entities::leave(world, shot);
}
