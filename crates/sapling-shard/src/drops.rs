//! Items players throw out of their inventories and items broken blocks leave, as BDS's `ItemActor`
//! (docs/research/items.md): each falls and slides as a body (body.rs) and is picked up by a player
//! with room for it who walks over it, once its delay is over; it is gone after 6000 ticks.
// TODO: items do not merge, float in water or burn in lava, and are not saved: they go with their
// column, the one they were let go in. Whether a throw starts at the eye is unsettled.

use std::collections::HashMap;

use acacia_proto::packets::TakeItemEntity;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::{ChunkPos, ItemStack};

use crate::blocks::Dice;
use crate::body::{Body, Physics};
use crate::components::{InLayer, Layer, NetId, Position, Stage, Tick};
use crate::inventory::Inventory;
use crate::sync::Told;
use crate::{blocks, inventory, mobs};

/// Ticks before a thrown item can be picked up, by anyone.
const PICKUP_DELAY: u64 = 40;
/// Ticks before what a broken block left can be picked up.
const SPILL_DELAY: u64 = 10;
const LIFETIME: u32 = 6000;
/// Its box, wide and high; the wire names the middle of it.
const SIZE: f32 = 0.25;
/// A broken block's items appear this far into its cell or more, on every axis.
const SPILL_INSET: f32 = 0.15;
/// What a block leaves is tossed up this fast, and this far either way across.
const SPILL_UP: f32 = 0.2;
const SPILL_ACROSS: f32 = 0.1;
/// A throw starts this far under the eye, at this speed along the look, give or take this much.
const THROW_BELOW_EYE: f32 = 0.5;
const THROW_SPEED: f32 = 0.3;
const THROW_SPREAD: f32 = 0.02;
/// A player picks up what touches its box grown by this much across and up and down (the Java
/// game's `Player.aiStep`; Bedrock's was not looked up).
const REACH: [f32; 2] = [1.0, 0.5];
const PLAYER_HALF_WIDTH: f32 = 0.3;
const PLAYER_HEIGHT: f32 = 1.8;

#[derive(Component)]
pub(crate) struct Dropped {
    stack: ItemStack,
    body: Body,
    told: Told,
    pickup_from: u64,
    age: u32,
}

fn wire([x, y, z]: [f32; 3]) -> [f32; 3] {
    [x, y + SIZE / 2.0, z]
}

fn dice(world: &mut World) -> Mut<'_, Dice> {
    world.get_resource_or_insert_with(Dice::default)
}

/// Lets `stack` go with the bottom of its box at `at`, moving at `motion`.
fn lay(world: &mut World, layer: Entity, stack: ItemStack, at: [f32; 3], motion: [f32; 3], pickup_from: u64) {
    let shown = mobs::spawn_shown(world, layer, at, |id| mobs::item_announcement(id, &stack, wire(at), motion));
    match shown {
        Some(item) => drop(world.entity_mut(item).insert(Dropped { stack, body: Body::new(at, motion, [SIZE; 2]), told: Told::new(wire(at)), pickup_from, age: 0 })),
        None => tracing::debug!(item = %stack.name, "an item no client knows; it is gone"),
    }
}

/// Throws `stack` from under the player's eye along its look (BDS `Player::drop`).
pub(crate) fn throw(world: &mut World, player: Entity, stack: ItemStack) {
    let (Some(position), Some(layer)) = (world.get::<Position>(player).copied(), world.get::<InLayer>(player).map(|layer| layer.0)) else { return };
    let ((sin_yaw, cos_yaw), (sin_pitch, cos_pitch)) = (position.yaw.to_radians().sin_cos(), position.pitch.to_radians().sin_cos());
    let (angle, spread) = { let mut dice = dice(world); (dice.fraction() * std::f32::consts::TAU, dice.fraction() * THROW_SPREAD) };
    let motion = [-sin_yaw * cos_pitch * THROW_SPEED + angle.cos() * spread, -sin_pitch * THROW_SPEED, cos_yaw * cos_pitch * THROW_SPEED + angle.sin() * spread];
    let feet = position.feet;
    let at = [feet[0], feet[1] + sapling_data::EYE_HEIGHT - THROW_BELOW_EYE - SIZE / 2.0, feet[2]];
    let from = world.resource::<Tick>().0 + PICKUP_DELAY;
    lay(world, layer, stack, at, motion, from);
}

/// Tosses `stack` out of the cell a broken block left (BDS `Spawner::spawnItem`).
pub(crate) fn spill(world: &mut World, layer: Entity, stack: ItemStack, pos: BlockPos) {
    let mut dice = dice(world);
    let at: [f32; 3] = std::array::from_fn(|axis| pos[axis] as f32 + SPILL_INSET + dice.fraction() * (1.0 - 2.0 * SPILL_INSET));
    let motion = [dice.fraction() * 2.0 * SPILL_ACROSS - SPILL_ACROSS, SPILL_UP, dice.fraction() * 2.0 * SPILL_ACROSS - SPILL_ACROSS];
    let from = world.resource::<Tick>().0 + SPILL_DELAY;
    lay(world, layer, stack, [at[0], at[1] - SIZE / 2.0, at[2]], motion, from);
}

/// Where each lying item is, with its world.
pub(crate) fn lying(world: &mut World) -> Vec<(Entity, [f32; 3])> {
    world.query::<(&Dropped, &InLayer)>().iter(world).map(|(item, layer)| (layer.0, item.body.at)).collect()
}

/// The items lying in `layer` inside the box from `min` to `max`, oldest first.
pub(crate) fn lying_in(world: &mut World, layer: Entity, min: [f32; 3], max: [f32; 3]) -> Vec<(Entity, ItemStack)> {
    let inside = |at: &[f32; 3]| (0..3).all(|axis| (min[axis]..=max[axis]).contains(&at[axis]));
    let mut lying = world.query::<(Entity, &Dropped, &InLayer, &NetId)>();
    let mut found: Vec<_> = lying.iter(world).filter(|(_, item, in_layer, _)| in_layer.0 == layer && inside(&item.body.at)).map(|(entity, item, _, id)| (id.0, entity, item.stack.clone())).collect();
    found.sort_unstable_by_key(|(id, ..)| *id);
    found.into_iter().map(|(_, entity, stack)| (entity, stack)).collect()
}

/// Takes `count` of the lying item `item` away; what is left lies on as a new item.
pub(crate) fn take(world: &mut World, item: Entity, count: u8) {
    let Some((stack, at, layer, pickup_from)) = world.query::<(&Dropped, &InLayer)>().get(world, item).ok().map(|(item, layer)| (item.stack.clone(), item.body.at, layer.0, item.pickup_from)) else { return };
    mobs::remove(world, item);
    if stack.count > count {
        lay(world, layer, ItemStack { count: stack.count - count, ..stack }, at, [0.0; 3], pickup_from);
    }
}

/// Moves every item a tick and lets go of those whose time is up.
pub(crate) fn fly(world: &mut World) {
    let items: Vec<(Entity, Entity, u64)> = world.query::<(Entity, &Dropped, &InLayer, &NetId)>().iter(world).map(|(entity, _, layer, id)| (entity, layer.0, id.0)).collect();
    for (entity, layer, id) in items {
        let Some(mut item) = world.entity_mut(entity).take::<Dropped>() else { continue };
        item.age += 1;
        if item.age >= LIFETIME {
            mobs::remove(world, entity);
            continue;
        }
        if let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) {
            item.body.step(level, &Physics::ITEM);
        }
        let packets = item.told.update(id, wire(item.body.at), item.body.motion, item.body.on_ground);
        if !packets.is_empty() {
            blocks::to_viewers(world, layer, item.body.at.map(|c| c.floor() as i32), &packets);
            if let Some(shown) = mobs::item_announcement(id, &item.stack, wire(item.body.at), item.body.motion) {
                mobs::reannounce(world, entity, shown);
            }
        }
        world.entity_mut(entity).insert(item);
    }
}

fn column(at: [f32; 3]) -> ChunkPos {
    ChunkPos::of_block(at[0].floor() as i32, at[2].floor() as i32)
}

/// Whether the item's box, its bottom at `item`, touches the reach of a player standing at `feet`.
fn reaches(feet: [f32; 3], item: [f32; 3]) -> bool {
    let across = PLAYER_HALF_WIDTH + REACH[0] + SIZE / 2.0;
    let up = item[1] - feet[1];
    (item[0] - feet[0]).abs() <= across && (item[2] - feet[2]).abs() <= across && up >= -REACH[1] - SIZE && up <= PLAYER_HEIGHT + REACH[1]
}

/// Gives each item that can be picked up to a player standing on it who has room, whole or as
/// much as fits; what is left lies on as a new item.
pub(crate) fn pick_up(world: &mut World) {
    let tick = world.resource::<Tick>().0;
    let mut lying = world.query::<(Entity, &Dropped, &InLayer, &NetId)>();
    let ready: Vec<_> = lying.iter(world).filter(|(_, item, ..)| item.pickup_from <= tick).map(|(entity, item, layer, id)| (entity, item.stack.clone(), item.body.at, layer.0, id.0)).collect();
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
            lay(world, layer, ItemStack { count: left, ..stack }, at, [0.0; 3], tick);
        }
    }
}
