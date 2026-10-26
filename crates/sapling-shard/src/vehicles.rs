//! Boats and minecarts (docs/research/vehicles.md): entities placed from their items, moved by
//! their own physics (boats.rs, carts.rs) unless a driver's client moves them (riding/drive.rs),
//! hit until they break into their item, carrying goods (cargo.rs) and pulling in mobs that bump
//! into them (pull.rs). Who rides them is riding.rs's.
// TODO: lava and fire do not break them.

mod boats;
mod cargo;
mod carts;
mod pull;
mod rails;

use acacia_proto::types::{MetadataDictionaryItem as Item, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use acacia_proto::packets::SetEntityData;
use acacia_proto::types::{EntityProperties, MetadataFlags1};
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_api::{BlockPos, Face, GameMode};
use sapling_data::AddEntityOptions;
use sapling_world::{Actor, ItemStack};

use crate::body::Body;
use crate::components::{InLayer, Mode, NetId, Position};
use crate::entities::{self, Kind};
use crate::inventory::Inventory;
use crate::movement::Motion;
use crate::sync::Told;

pub(crate) use cargo::{Cargo, cargo, load_cargo};

/// Riders who asked for their inventory while riding a vehicle with goods: that opens instead
/// (inventory/equipment.rs takes the asks in).
// TODO: unverified that BDS opens a chest boat's goods to its rider's inventory key.
#[derive(Resource, Default)]
pub(crate) struct CargoAsked(pub Vec<(Entity, Entity)>);

pub(crate) const BOAT: &str = "minecraft:boat";
pub(crate) const CHEST_BOAT: &str = "minecraft:chest_boat";
pub(crate) const MINECART: &str = "minecraft:minecart";
pub(crate) const TNT_MINECART: &str = "minecraft:tnt_minecart";
const CARTS: [&str; 4] = [MINECART, "minecraft:chest_minecart", "minecraft:hopper_minecart", TNT_MINECART];
/// Structural integrity, shown as `Health`; a hit takes this many times its damage off a boat, a
/// minecart (docs/research/vehicles.md "Breaking").
const INTEGRITY: i32 = 40;
const BOAT_HIT: f32 = 10.0;
const CART_HIT: f32 = 8.0;
/// A hit shows this many ticks of shaking.
const HURT_TICKS: i32 = 10;
/// A minecart mends only below this.
const CART_MENDS_BELOW: i32 = 20;
const ENTITY_DROPS: &str = "doEntityDrops";
/// A boat's `Variant` by wood (BDS's drop switch; docs/research/vehicles.md "Boats").
const WOODS: [&str; 10] = ["oak", "spruce", "birch", "jungle", "acacia", "dark_oak", "mangrove", "bamboo", "cherry", "pale_oak"];

pub(crate) fn is_vehicle(kind: &str) -> bool {
    kind == BOAT || kind == CHEST_BOAT || CARTS.contains(&kind)
}

fn is_boat(kind: &str) -> bool {
    kind == BOAT || kind == CHEST_BOAT
}

/// A boat's or minecart's own part: its look (a boat's wood) and how hurt it is.
#[derive(Component, Clone, Copy)]
pub(crate) struct Vehicle {
    pub variant: i32,
    integrity: i32,
    hurt: i32,
    side: i32,
}

/// What shows a vehicle: BDS's announcement of its kind, in its wood, with its integrity.
fn announcement(id: u64, kind: &str, wire: [f32; 3], yaw: f32, vehicle: &Vehicle) -> Vec<Bytes> {
    let options = AddEntityOptions { id, identifier: kind, feet: wire, yaw, pitch: 0.0, variant: vehicle.variant, mark_variant: 0, color: 0, baby: false, flags: MetadataFlags1(0), health: None, name: "" };
    let mut added = sapling_data::add_entity(&options);
    added.metadata.retain(|item| !matches!(item.key, Key::Health | Key::HurtTime | Key::HurtDirection));
    added.metadata.extend(hurt_items(vehicle));
    vec![sapling_net::encode(&added)]
}

fn hurt_items(vehicle: &Vehicle) -> [Item; 3] {
    let int = |key, value| Item { key, r#type: Type::Int, legacy_type: 2, value: Value::Default(Plain::Int(value)) };
    [int(Key::Health, vehicle.integrity), int(Key::HurtTime, vehicle.hurt), int(Key::HurtDirection, vehicle.side)]
}

/// Puts a vehicle of `kind` into `layer`, its box's bottom at `feet`, facing `yaw`, of `variant`;
/// `kept` is what a world folder kept of it, or `None` for a new one.
fn spawn(world: &mut World, layer: Entity, kind: &str, feet: [f32; 3], yaw: f32, variant: i32, kept: Option<(i64, Vec<u8>)>) -> Option<Entity> {
    let size = sapling_data::mob_size(kind)?;
    let lift = crate::riding::wire_lift(kind);
    let wire = [feet[0], feet[1] + lift, feet[2]];
    let vehicle = Vehicle { variant, integrity: INTEGRITY, hurt: 0, side: 1 };
    let body = Body::new(feet, [0.0; 3], size);
    let entity = entities::spawn_mob(world, layer, kind, body, wire, |id| Some(announcement(id, kind, wire, yaw, &vehicle)))?;
    world.entity_mut(entity).insert(vehicle);
    if let Some(mut told) = world.get_mut::<Told>(entity) {
        told.face(yaw, yaw);
    }
    let (unique_id, compound) = match kept {
        Some(kept) => kept,
        None => {
            let unique_id = entities::new_unique_id(world, entity)?;
            let compound = sapling_world::new_actor(unique_id, kind, feet, [0.0; 3]);
            (unique_id, sapling_world::varied_actor(&compound, variant, yaw).ok()?)
        }
    };
    entities::keep(world, entity, layer, unique_id, compound);
    cargo::stow(world, entity, kind);
    if is_boat(kind) {
        boats::fit(world, entity, wire, yaw);
    } else {
        carts::fit(world, entity);
    }
    crate::riding::adopt(world, entity);
    Some(entity)
}

/// Summons a vehicle of `kind` (the `/summon`, `/ride` and API way) at `feet`.
pub(crate) fn summon(world: &mut World, layer: Entity, kind: &str, feet: [f32; 3], yaw: f32) -> Option<sapling_api::EntityId> {
    let vehicle = spawn(world, layer, kind, feet, yaw, 0, None)?;
    Some(sapling_api::EntityId(world.get::<NetId>(vehicle)?.0))
}

/// A player's click with `held` on `face` of the block at `pos`, `at` within it: a boat or
/// minecart item places its vehicle, spent outside creative. False where it places nothing.
pub(crate) fn place(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, face: Face, at: [f32; 3], held: &str) -> bool {
    let yaw = world.get::<Position>(player).map_or(0.0, |position| position.yaw);
    let placed = match wood_of(held) {
        Some(wood) => boats::place(world, layer, pos, face, at, yaw, if held.contains("_chest_") { CHEST_BOAT } else { BOAT }, wood),
        None if CARTS.contains(&held) => carts::place(world, layer, pos, held),
        None => false,
    };
    if placed {
        crate::blocks::spend_held(world, player);
    }
    placed
}

/// Whether a vehicle's box with these `bounds` would overlap a mob or another vehicle.
fn is_crowded(world: &mut World, layer: Entity, (low, high): ([f32; 3], [f32; 3])) -> bool {
    let mut bodies = world.query_filtered::<(&Body, &InLayer, &Kind), Or<(With<Vehicle>, With<crate::creatures::Creature>)>>();
    bodies.iter(world).filter(|(_, in_layer, _)| in_layer.0 == layer).any(|(body, ..)| {
        let (other_low, other_high) = body.bounds();
        (0..3).all(|axis| low[axis] < other_high[axis] && high[axis] > other_low[axis])
    })
}

/// Makes the stored vehicle `actor` an entity; false for a kind that is no vehicle.
// TODO: unverified whether BDS keeps a boat's `Pos` at its feet or where the wire puts it; feet here.
pub(crate) fn from_storage(world: &mut World, layer: Entity, actor: &Actor, unique_id: i64, compound: Vec<u8>) -> bool {
    is_vehicle(&actor.identifier) && spawn(world, layer, &actor.identifier, actor.position, actor.yaw, actor.variant, Some((unique_id, compound))).is_some()
}

/// A tick of every vehicle: boats float and drift, minecarts roll, both mend, and they pull in
/// mobs.
pub(crate) fn tick(world: &mut World) {
    for (player, vehicle) in std::mem::take(&mut world.get_resource_or_insert_with(CargoAsked::default).0) {
        cargo::open(world, player, vehicle);
    }
    boats::tick(world);
    carts::tick(world);
    cargo::take_up(world);
    pull::tick(world);
    let mending: Vec<(Entity, Vehicle, bool)> = world.query::<(Entity, &Vehicle, &Kind)>().iter(world).filter(|(_, vehicle, _)| vehicle.hurt > 0 || vehicle.integrity < INTEGRITY).map(|(entity, vehicle, kind)| (entity, *vehicle, is_boat(&kind.0))).collect();
    for (entity, mut vehicle, boat) in mending {
        vehicle.hurt = (vehicle.hurt - 1).max(0);
        if vehicle.integrity >= 1 && vehicle.integrity < if boat { INTEGRITY } else { CART_MENDS_BELOW } {
            vehicle.integrity += 1;
        }
        world.entity_mut(entity).insert(vehicle);
    }
}

/// A player's click on a vehicle, once plugins let it through: a chest minecart or hopper
/// minecart opens, a chest boat opens to a sneaking player, flint and steel primes a TNT
/// minecart. False where none of these is so: the player may get on (riding.rs).
pub(crate) fn used_on(world: &mut World, player: Entity, vehicle: Entity) -> bool {
    let Some(kind) = world.get::<Kind>(vehicle).map(|kind| kind.0.clone()).filter(|_| crate::riding::reaches(world, player, vehicle)) else { return false };
    let sneaking = world.get::<Motion>(player).is_some_and(|motion| motion.pose.sneaking);
    let held = world.get::<Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref().map(|held| held.stack.name.clone()));
    match kind.as_str() {
        TNT_MINECART if held.as_deref() == Some("minecraft:flint_and_steel") => carts::prime(world, vehicle, carts::FUSE),
        CHEST_BOAT if sneaking => cargo::open(world, player, vehicle),
        "minecraft:chest_minecart" | "minecraft:hopper_minecart" => cargo::open(world, player, vehicle),
        _ => false,
    }
}

/// A player hit `vehicle` for `damage`: it shakes, and breaks once its integrity is gone, at
/// once for a creative player, who gets nothing of it.
pub(crate) fn hit(world: &mut World, player: Entity, vehicle: Entity, damage: f32) {
    let (Some(mut state), Some(kind), Some(id)) = (world.get::<Vehicle>(vehicle).copied(), world.get::<Kind>(vehicle).map(|kind| kind.0.clone()), world.get::<NetId>(vehicle).map(|id| id.0)) else { return };
    let creative = world.get::<Mode>(player).is_some_and(|mode| mode.0 == GameMode::Creative);
    let per_damage = if is_boat(&kind) { BOAT_HIT } else { CART_HIT };
    (state.side, state.hurt) = (-state.side, HURT_TICKS);
    state.integrity -= (damage * per_damage) as i32;
    world.entity_mut(vehicle).insert(state);
    let shown = SetEntityData { runtime_entity_id: id, metadata: hurt_items(&state).to_vec(), properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 };
    if let (Some(layer), Some(body)) = (world.get::<InLayer>(vehicle).map(|layer| layer.0), world.get::<Body>(vehicle)) {
        let cell = body.at.map(|c| c.floor() as i32);
        entities::tell_viewers(world, vehicle, layer, cell, vec![sapling_net::encode(&shown)]);
    }
    if creative || state.integrity <= 0 {
        wreck(world, vehicle, !creative);
    }
}

/// A blast at `at` broke the vehicles within `reach` of it; a TNT minecart is primed instead, on a
/// short fuse.
// TODO: BDS hurts a vehicle by the blast's damage; here one in reach breaks.
pub(crate) fn blasted(world: &mut World, layer: Entity, at: [f32; 3], reach: f32) {
    let near: Vec<(Entity, bool)> = world
        .query::<(Entity, &InLayer, &Body, &Kind, &Vehicle)>()
        .iter(world)
        .filter(|(_, in_layer, body, ..)| in_layer.0 == layer && (0..3).map(|axis| (body.at[axis] - at[axis]).powi(2)).sum::<f32>() < reach * reach)
        .map(|(vehicle, _, _, kind, _)| (vehicle, kind.0 == TNT_MINECART))
        .collect();
    for (vehicle, is_tnt) in near {
        if is_tnt {
            let short = carts::SHORT_FUSE[0] + world.get_resource_or_insert_with(crate::blocks::Dice::default).roll(carts::SHORT_FUSE[1] as u32) as i32;
            carts::prime(world, vehicle, short);
        } else {
            wreck(world, vehicle, true);
        }
    }
}

/// The item a vehicle of `kind` in `variant` breaks into, and that places it.
fn item_of(kind: &str, variant: i32) -> String {
    let wood = WOODS.get(usize::try_from(variant).unwrap_or(0)).copied().unwrap_or("oak");
    match (kind, wood) {
        (BOAT, "bamboo") => "minecraft:bamboo_raft".to_owned(),
        (CHEST_BOAT, "bamboo") => "minecraft:bamboo_chest_raft".to_owned(),
        (BOAT, wood) => format!("minecraft:{wood}_boat"),
        (CHEST_BOAT, wood) => format!("minecraft:{wood}_chest_boat"),
        (kind, _) => kind.to_owned(),
    }
}

/// The wood a boat item is made of; `None` for no boat item. A chest one is told by its name.
fn wood_of(item: &str) -> Option<i32> {
    let name = item.strip_prefix("minecraft:")?.replace("_raft", "_boat");
    let wood = name.strip_suffix("_chest_boat").or_else(|| name.strip_suffix("_boat"))?;
    WOODS.iter().position(|known| *known == wood).map(|at| at as i32)
}

/// Breaks `vehicle`: its riders get off, what it carries spills, and with `drops` (and
/// `doEntityDrops` on) it leaves its item.
pub(crate) fn wreck(world: &mut World, vehicle: Entity, drops: bool) {
    let (Some(kind), Some(state), Some(layer), Some(body)) =
        (world.get::<Kind>(vehicle).map(|kind| kind.0.clone()), world.get::<Vehicle>(vehicle).copied(), world.get::<InLayer>(vehicle).map(|layer| layer.0), world.get::<Body>(vehicle).copied())
    else {
        return;
    };
    crate::riding::forget(world, vehicle);
    cargo::spill(world, vehicle);
    if drops && crate::rules::on(world, layer, ENTITY_DROPS) {
        let stack = ItemStack { name: item_of(&kind, state.variant), count: 1, damage: 0, block: None, tag: None };
        crate::drops::let_go(world, layer, stack, body.at);
    }
    crate::mobs::remove(world, vehicle);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_boat_breaks_into_the_item_of_its_wood_and_the_item_places_that_wood() {
        assert_eq!(item_of(BOAT, 2), "minecraft:birch_boat");
        assert_eq!(item_of(CHEST_BOAT, 7), "minecraft:bamboo_chest_raft");
        assert_eq!(item_of(MINECART, 0), MINECART);
        for (item, wood) in [("minecraft:birch_boat", 2), ("minecraft:bamboo_raft", 7), ("minecraft:pale_oak_chest_boat", 9), ("minecraft:dark_oak_boat", 5)] {
            assert_eq!(wood_of(item), Some(wood), "{item}");
        }
        assert_eq!(wood_of("minecraft:oak_planks"), None);
    }
}
