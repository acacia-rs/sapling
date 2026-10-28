//! Items players throw out of their inventories and items broken blocks leave, as BDS's `ItemActor`
//! (docs/research/items.md): each falls and slides as a body (body.rs) and is picked up by a player
//! with room for it who walks over it, once its delay is over; it is gone after 6000 ticks, or
//! burnt in lava. Items of a kind lying together join up (drops/merge.rs).
// TODO: whether a throw starts at the eye is unsettled.

mod merge;

use std::collections::HashMap;

use acacia_proto::packets::TakeItemEntity;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::{ChunkPos, ItemStack};

use crate::blocks::Dice;
use crate::body::{Body, Liquid, Physics};
use crate::components::{InLayer, Layer, Mode, NetId, Position, Stage, Tick};
use crate::inventory::Inventory;
use crate::{blocks, entities, inventory, mobs};

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
/// Lava burns an item away in this many ticks: its 5 health, 4 a tick. BDS's went the tick after,
/// without moving again (research/captures/bds-lava.txt).
const BURNT_IN: u8 = 2;

/// An item's own part; its [`Body`] and what viewers were [`Told`] are beside it (entities.rs).
#[derive(Component)]
pub(crate) struct Dropped {
    stack: ItemStack,
    pickup_from: u64,
    age: u32,
    /// Ticks it has spent in lava.
    burnt: u8,
}

impl Dropped {
    pub(crate) fn stack(&self) -> &ItemStack {
        &self.stack
    }
}

/// Netherite and ancient debris, which lava does not burn.
fn fireproof(name: &str) -> bool {
    name.strip_prefix("minecraft:").is_some_and(|name| FIREPROOF.contains(&name))
}

/// The items BDS flags fire resistant (docs/research/assumptions-checked.md 5).
const FIREPROOF: [&str; 16] = [
    "netherite_sword",
    "netherite_shovel",
    "netherite_pickaxe",
    "netherite_axe",
    "netherite_hoe",
    "netherite_spear",
    "netherite_ingot",
    "netherite_scrap",
    "netherite_helmet",
    "netherite_chestplate",
    "netherite_leggings",
    "netherite_boots",
    "netherite_horse_armor",
    "netherite_nautilus_armor",
    "netherite_block",
    "ancient_debris",
];

const KIND: &str = "minecraft:item";

fn wire([x, y, z]: [f32; 3]) -> [f32; 3] {
    [x, y + SIZE / 2.0, z]
}

fn dice(world: &mut World) -> Mut<'_, Dice> {
    world.get_resource_or_insert_with(Dice::default)
}

/// Lets `stack` go with the bottom of its box at `at`, moving at `motion`, for the world folder
/// to keep with the column it lies in (actors.rs).
fn lay(world: &mut World, layer: Entity, stack: ItemStack, at: [f32; 3], motion: [f32; 3], pickup_from: u64) -> Option<Entity> {
    lay_as(world, layer, stack, at, motion, pickup_from, false)
}

/// [`lay`], shown as a hook's catch when `fished`.
fn lay_as(world: &mut World, layer: Entity, stack: ItemStack, at: [f32; 3], motion: [f32; 3], pickup_from: u64, fished: bool) -> Option<Entity> {
    let item = lay_unkept(world, layer, stack.clone(), at, motion, pickup_from, fished)?;
    let unique_id = entities::new_unique_id(world, item)?;
    let compound = sapling_world::new_item_actor(unique_id, &stack, world.get::<Layer>(layer)?.world.registry(), at, motion);
    entities::keep(world, item, layer, unique_id, compound);
    Some(item)
}

fn lay_unkept(world: &mut World, layer: Entity, stack: ItemStack, at: [f32; 3], motion: [f32; 3], pickup_from: u64, fished: bool) -> Option<Entity> {
    let shown = entities::spawn(world, layer, KIND, Body::new(at, motion, [SIZE; 2]), wire(at), |id| mobs::item_announcement(id, &stack, wire(at), motion, fished));
    match shown {
        Some(item) => {
            entities::flag_moving(world, item, false);
            Some(world.entity_mut(item).insert(Dropped { stack, pickup_from, age: 0, burnt: 0 }).id())
        }
        None => {
            tracing::debug!(item = %stack.name, "an item no client knows; it is gone");
            None
        }
    }
}

/// Makes an item a world folder kept (its actor compound `compound`) lie in `layer` again, to be
/// picked up at once; false for one no client knows, which stays in the folder as it was.
pub(crate) fn from_storage(world: &mut World, layer: Entity, stack: ItemStack, at: [f32; 3], unique_id: i64, compound: Vec<u8>) -> bool {
    let tick = world.resource::<Tick>().0;
    let Some(item) = lay_unkept(world, layer, stack, at, [0.0; 3], tick, false) else { return false };
    entities::keep(world, item, layer, unique_id, compound);
    true
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

/// Tosses `stack` up from `centre` (the middle of its box, where the wire puts it), as BDS's
/// `Spawner::spawnItem` does every item it makes.
pub(crate) fn toss(world: &mut World, layer: Entity, stack: ItemStack, centre: [f32; 3]) {
    let mut dice = dice(world);
    let motion = [dice.fraction() * 2.0 * SPILL_ACROSS - SPILL_ACROSS, SPILL_UP, dice.fraction() * 2.0 * SPILL_ACROSS - SPILL_ACROSS];
    let from = world.resource::<Tick>().0 + SPILL_DELAY;
    lay(world, layer, stack, [centre[0], centre[1] - SIZE / 2.0, centre[2]], motion, from);
}

/// Shoots `stack` out from `centre` (the middle of its box) at `motion`, picked up as soon as what
/// a broken block leaves: a dispenser's shot, a trial spawner's or a vault's reward.
pub(crate) fn eject(world: &mut World, layer: Entity, stack: ItemStack, centre: [f32; 3], motion: [f32; 3]) {
    let from = world.resource::<Tick>().0 + SPILL_DELAY;
    lay(world, layer, stack, [centre[0], centre[1] - SIZE / 2.0, centre[2]], motion, from);
}

/// A hook's catch: `stack` flies from the hook at `at` (the bottom of its box) at `motion`.
pub(crate) fn reeled(world: &mut World, layer: Entity, stack: ItemStack, at: [f32; 3], motion: [f32; 3]) {
    let from = world.resource::<Tick>().0 + SPILL_DELAY;
    lay_as(world, layer, stack, at, motion, from, true);
}

/// Tosses `stack` out of somewhere in the cell a broken block left.
pub(crate) fn spill(world: &mut World, layer: Entity, stack: ItemStack, pos: BlockPos) {
    let mut dice = dice(world);
    let centre: [f32; 3] = std::array::from_fn(|axis| pos[axis] as f32 + SPILL_INSET + dice.fraction() * (1.0 - 2.0 * SPILL_INSET));
    toss(world, layer, stack, centre);
}

/// Tosses `stack` out of where an entity that broke up was, its box's bottom at `at`, the entity's
/// position on the wire (BDS `FallingBlockActor::breakBlock`, research/captures/).
pub(crate) fn let_go(world: &mut World, layer: Entity, stack: ItemStack, [x, y, z]: [f32; 3]) {
    toss(world, layer, stack, [x, y + SIZE / 2.0, z]);
}

/// The items lying in `layer` inside the box from `min` to `max`, oldest first.
pub(crate) fn lying_in(world: &mut World, layer: Entity, min: [f32; 3], max: [f32; 3]) -> Vec<(Entity, ItemStack)> {
    let inside = |at: &[f32; 3]| (0..3).all(|axis| (min[axis]..=max[axis]).contains(&at[axis]));
    let mut lying = world.query::<(Entity, &Dropped, &Body, &InLayer, &NetId)>();
    let mut found: Vec<_> = lying.iter(world).filter(|(_, _, body, in_layer, _)| in_layer.0 == layer && inside(&body.at)).map(|(entity, item, _, _, id)| (id.0, entity, item.stack.clone())).collect();
    found.sort_unstable_by_key(|(id, ..)| *id);
    found.into_iter().map(|(_, entity, stack)| (entity, stack)).collect()
}

/// Takes `count` of the lying item `item` away; what is left lies on as a new item.
pub(crate) fn take(world: &mut World, item: Entity, count: u8) {
    let Some((stack, at, layer, pickup_from)) = world.query::<(&Dropped, &Body, &InLayer)>().get(world, item).ok().map(|(item, body, layer)| (item.stack.clone(), body.at, layer.0, item.pickup_from)) else { return };
    mobs::remove(world, item);
    if stack.count > count {
        lay(world, layer, ItemStack { count: stack.count - count, ..stack }, at, [0.0; 3], pickup_from);
    }
}

/// Moves every item a tick and lets go of those whose time is up, or that lava burnt.
pub(crate) fn fly(world: &mut World) {
    let items: Vec<(Entity, Entity, u64)> = world.query::<(Entity, &Dropped, &InLayer, &NetId)>().iter(world).map(|(entity, _, layer, id)| (entity, layer.0, id.0)).collect();
    for (entity, layer, id) in items {
        let Some(mut item) = world.get_mut::<Dropped>(entity) else { continue };
        item.age += 1;
        if item.age >= LIFETIME || item.burnt >= BURNT_IN {
            mobs::remove(world, entity);
            continue;
        }
        let (physics, age) = (if fireproof(&item.stack.name) { Physics::FIREPROOF_ITEM } else { Physics::ITEM }, item.age - 1);
        merge::join(world, entity, layer, age);
        if world.get::<Dropped>(entity).is_none() {
            continue;
        }
        lift_out(world, entity, layer);
        let Some((body, packets)) = entities::step(world, entity, layer, id, &physics, SIZE / 2.0) else { continue };
        let Some(mut item) = world.get_mut::<Dropped>(entity) else { continue };
        if physics.burns && body.liquid() == Some(Liquid::Lava) {
            item.burnt += 1;
            if item.burnt >= BURNT_IN {
                continue;
            }
        }
        if !packets.is_empty() {
            entities::tell_viewers(world, entity, layer, body.at.map(|c| c.floor() as i32), packets);
            let shown = world.get::<Dropped>(entity).and_then(|item| mobs::item_announcement(id, &item.stack, wire(body.at), body.motion, false));
            shown.into_iter().for_each(|shown| mobs::reannounce(world, entity, shown));
        }
    }
}

fn lift_out(world: &mut World, entity: Entity, layer: Entity) {
    let (Some(level), Some(mut body)) = (world.get::<Layer>(layer), world.get::<Body>(entity).copied()) else { return };
    body.lift_out(&level.world);
    if let Some(mut stored) = world.get_mut::<Body>(entity) {
        *stored = body;
    }
}

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
