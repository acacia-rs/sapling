//! Players picking up lying items.

use std::collections::HashMap;

use acacia_proto::packets::TakeItemEntity;
use bevy_ecs::prelude::*;
use sapling_world::{ChunkPos, ItemStack};

use super::{Dropped, SIZE, lay};
use crate::body::Body;
use crate::components::{InLayer, Mode, NetId, Position, Stage, Tick};
use crate::inventory::Inventory;
use crate::{blocks, entities, inventory};

/// A player picks up what touches its box grown by this much across and up and down (the Java
/// game's `Player.aiStep`; Bedrock's was not looked up).
const REACH: [f32; 2] = [1.0, 0.5];
const PLAYER_HALF_WIDTH: f32 = 0.3;
const PLAYER_HEIGHT: f32 = 1.8;

fn column(at: [f32; 3]) -> ChunkPos {
    ChunkPos::of_block(at[0].floor() as i32, at[2].floor() as i32)
}

/// Whether a box `size` wide and high, its bottom at `at`, touches the reach of a player standing at
/// `feet`: BDS's pickup area for items and orbs (docs/research/assumptions-checked.md 3).
pub(crate) fn within_pickup(feet: [f32; 3], at: [f32; 3], size: f32) -> bool {
    let across = PLAYER_HALF_WIDTH + REACH[0] + size / 2.0;
    let up = at[1] - feet[1];
    (at[0] - feet[0]).abs() <= across && (at[2] - feet[2]).abs() <= across && up >= -REACH[1] - size && up <= PLAYER_HEIGHT + REACH[1]
}

/// Whether a player in `mode` takes up items and orbs: a spectator does neither
/// (research/captures/bds-spectate.txt).
pub(crate) fn takes_up(mode: Option<&Mode>) -> bool {
    mode.is_none_or(|mode| mode.0 != sapling_api::GameMode::Spectator)
}

/// Gives each item that can be picked up to a player standing on it who has room, whole or as
/// much as fits; what is left lies on as a new item.
pub(crate) fn pick_up(world: &mut World) {
    let tick = world.resource::<Tick>().0;
    let mut lying = world.query::<(Entity, &Dropped, &Body, &InLayer, &NetId)>();
    let ready: Vec<_> = lying.iter(world).filter(|(_, item, ..)| item.pickup_from <= tick).map(|(entity, item, body, layer, id)| (entity, item.stack.clone(), body.at, layer.0, id.0)).collect();
    if ready.is_empty() {
        return;
    }
    let mut players = world.query::<(Entity, &Stage, &InLayer, &Position, &NetId, Option<&Mode>)>();
    let mut standing: HashMap<(Entity, ChunkPos), Vec<(Entity, [f32; 3], u64)>> = HashMap::new();
    for (player, stage, layer, position, id, mode) in players.iter(world) {
        if *stage == Stage::InWorld && takes_up(mode) {
            standing.entry((layer.0, column(position.feet))).or_default().push((player, position.feet, id.0));
        }
    }
    for (item, stack, at, layer, item_id) in ready {
        let home = column(at);
        let around = (-1..=1).flat_map(|dx| (-1..=1).map(move |dz| ChunkPos::new(home.x + dx, home.z + dz)));
        let near = around.filter_map(|pos| standing.get(&(layer, pos))).flatten().filter(|(_, feet, _)| within_pickup(*feet, at, SIZE));
        let taker = near.copied().find(|(player, ..)| world.get::<Inventory>(*player).is_some_and(|inventory| inventory.loaded && inventory.has_room_for(&stack)));
        let Some((player, _, player_id)) = taker.filter(|(player, ..)| crate::life_events::may_pick_up(world, *player, &stack)) else { continue };
        let left = inventory::change(world, player, |inventory| inventory.add(&stack)).unwrap_or(stack.count);
        let taken = sapling_net::encode(&TakeItemEntity { runtime_entity_id: item_id, target: player_id as u32 });
        blocks::to_viewers(world, layer, at.map(|c| c.floor() as i32), &[taken]);
        world.entity_mut(item).remove::<Dropped>();
        entities::leave(world, item);
        if left > 0 {
            lay(world, layer, ItemStack { count: left, ..stack }, at, [0.0; 3], tick);
        }
    }
}
