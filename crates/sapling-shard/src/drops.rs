//! Items players throw out of their inventories and items broken blocks leave: each lies where
//! it landed, shown like a stored item (mobs.rs), until a player with room for it walks over it.
// TODO: an item lands at once and never moves again (a block broken under it leaves it in the
// air), and is not saved: it goes with its column.

use std::collections::HashMap;

use acacia_proto::packets::TakeItemEntity;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::{ChunkPos, ItemStack};

use crate::components::{InLayer, Layer, NetId, Position, Stage, Tick};
use crate::inventory::Inventory;
use crate::{blocks, inventory, mobs};

/// Ticks before a thrown item can be picked up, so its thrower can walk away from it.
const PICKUP_DELAY: u64 = 40;
/// Ticks before what a broken block left can be picked up.
const SPILL_DELAY: u64 = 10;
/// The most cells an item falls before it lies.
const FALL: i32 = 64;
/// How far in front of its thrower an item lands.
const THROW: f32 = 1.5;
/// How far from a player's feet an item is picked up: across, below and above.
const REACH: [f32; 3] = [1.3, 0.5, 2.3];

#[derive(Component)]
pub(crate) struct Dropped {
    stack: ItemStack,
    at: [f32; 3],
    pickup_from: u64,
}

/// Where an item let go at `at` comes to rest: on the first solid block below it, or where it
/// is when there is none within [`FALL`] cells or the world ends first.
fn landing(world: &World, layer: Entity, at: [f32; 3]) -> [f32; 3] {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return at };
    let [x, y, z] = at.map(|c| c.floor() as i32);
    let is_solid = |y: i32| level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_solid());
    if !level.is_loaded(column(at)) || is_solid(y) {
        return at;
    }
    let floor = (1..=FALL).map(|down| y - down).find(|below| is_solid(*below));
    floor.filter(|floor| *floor != y - 1).map_or(at, |floor| [at[0], (floor + 1) as f32, at[2]])
}

fn lay(world: &mut World, layer: Entity, stack: ItemStack, at: [f32; 3], pickup_from: u64) {
    let at = landing(world, layer, at);
    match mobs::spawn_item(world, layer, &stack, at) {
        Some(item) => drop(world.entity_mut(item).insert(Dropped { stack, at, pickup_from })),
        None => tracing::debug!(item = %stack.name, "an item no client knows; it is gone"),
    }
}

/// Lays `stack` in front of the player who threw it.
pub(crate) fn throw(world: &mut World, player: Entity, stack: ItemStack) {
    let (Some(position), Some(layer)) = (world.get::<Position>(player).copied(), world.get::<InLayer>(player).map(|layer| layer.0)) else { return };
    let (sin, cos) = position.yaw.to_radians().sin_cos();
    let at = [position.feet[0] - sin * THROW, position.feet[1], position.feet[2] + cos * THROW];
    let from = world.resource::<Tick>().0 + PICKUP_DELAY;
    lay(world, layer, stack, at, from);
}

/// Lays `stack` in the cell a broken block left.
pub(crate) fn spill(world: &mut World, layer: Entity, stack: ItemStack, [x, y, z]: BlockPos) {
    let from = world.resource::<Tick>().0 + SPILL_DELAY;
    lay(world, layer, stack, [x as f32 + 0.5, y as f32, z as f32 + 0.5], from);
}

/// Where each lying item is, with its world.
pub(crate) fn lying(world: &mut World) -> Vec<(Entity, [f32; 3])> {
    world.query::<(&Dropped, &InLayer)>().iter(world).map(|(item, layer)| (layer.0, item.at)).collect()
}

fn column(at: [f32; 3]) -> ChunkPos {
    ChunkPos::of_block(at[0].floor() as i32, at[2].floor() as i32)
}

fn reaches(feet: [f32; 3], item: [f32; 3]) -> bool {
    let up = item[1] - feet[1];
    (item[0] - feet[0]).abs() <= REACH[0] && (item[2] - feet[2]).abs() <= REACH[0] && (-REACH[1]..=REACH[2]).contains(&up)
}

/// Gives each thrown item that can be picked up to a player standing on it who has room, whole
/// or as much as fits; what is left lies on as a new item.
pub(crate) fn pick_up(world: &mut World) {
    let tick = world.resource::<Tick>().0;
    let mut lying = world.query::<(Entity, &Dropped, &InLayer, &NetId)>();
    let ready: Vec<_> = lying.iter(world).filter(|(_, item, ..)| item.pickup_from <= tick).map(|(entity, item, layer, id)| (entity, item.stack.clone(), item.at, layer.0, id.0)).collect();
    if ready.is_empty() {
        return;
    }
    let mut players = world.query::<(Entity, &Stage, &InLayer, &Position, &NetId)>();
    let mut standing: HashMap<(Entity, ChunkPos), Vec<(Entity, [f32; 3], u64)>> = HashMap::new();
    for (player, stage, layer, position, id) in players.iter(world) {
        if *stage == Stage::InWorld {
            standing.entry((layer.0, column(position.feet))).or_default().push((player, position.feet, id.0));
        }
    }
    for (item, stack, at, layer, item_id) in ready {
        let home = column(at);
        let around = (-1..=1).flat_map(|dx| (-1..=1).map(move |dz| ChunkPos::new(home.x + dx, home.z + dz)));
        let near = around.filter_map(|pos| standing.get(&(layer, pos))).flatten().filter(|(_, feet, _)| reaches(*feet, at));
        let taker = near.copied().find(|(player, ..)| world.get::<Inventory>(*player).is_some_and(|inventory| inventory.loaded && inventory.has_room_for(&stack)));
        let Some((player, _, player_id)) = taker else { continue };
        let left = inventory::change(world, player, |inventory| inventory.add(&stack)).unwrap_or(stack.count);
        let taken = sapling_net::encode(&TakeItemEntity { runtime_entity_id: item_id, target: player_id as u32 });
        blocks::to_viewers(world, layer, at.map(|c| c.floor() as i32), &[taken]);
        mobs::remove(world, item);
        if left > 0 {
            lay(world, layer, ItemStack { count: left, ..stack }, at, tick);
        }
    }
}
