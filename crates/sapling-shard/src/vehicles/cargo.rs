//! What chest boats and chest and hopper minecarts carry (docs/research/vehicles.md "Cargo"):
//! slots kept on the vehicle and in its stored `ChestItems`, shown on a screen naming the
//! vehicle, spilled when it breaks; a hopper minecart takes up the items it rolls over.
// TODO: hoppers neither fill nor empty vehicles, comparators do not read them, and an
// activator rail does not lock a hopper minecart.

use acacia_proto::types::WindowType;
use bevy_ecs::prelude::*;
use sapling_world::ItemStack;

use crate::body::Body;
use crate::components::{InLayer, NetId, Tick};
use crate::entities::{Kind, Stored};
use crate::inventory::Held;
use crate::{containers, drops, entities, hoppers};

const HOPPER: &str = "minecraft:hopper_minecart";
/// A hopper minecart takes an item every this many ticks, from this far around its box's bottom.
const HOPPER_EVERY: u64 = 4;
const HOPPER_REACH: f32 = 0.5;

/// A vehicle's slots.
#[derive(Component, Clone)]
pub(crate) struct Cargo(Vec<Option<Held>>);

/// How many slots a vehicle of `kind` has, and the screen they show on.
fn hold(kind: &str) -> Option<(usize, WindowType)> {
    Some(match kind {
        "minecraft:chest_boat" => (27, WindowType::ChestBoat),
        "minecraft:chest_minecart" => (27, WindowType::MinecartChest),
        HOPPER => (5, WindowType::MinecartHopper),
        _ => return None,
    })
}

/// Gives a new vehicle of `kind` its empty slots, or those its stored compound holds.
pub(super) fn stow(world: &mut World, vehicle: Entity, kind: &str) {
    let Some((size, _)) = hold(kind) else { return };
    let stored = world.get::<Stored>(vehicle).map(|stored| sapling_world::actor_chest(&stored.compound)).unwrap_or_default();
    let mut slots = vec![None; size];
    for (slot, stack) in stored.into_iter().filter(|(slot, _)| usize::from(*slot) < size) {
        slots[usize::from(slot)] = Some(containers::new_held(world, stack));
    }
    world.entity_mut(vehicle).insert(Cargo(slots));
}

/// The slots of `vehicle`; `None` for one that carries nothing or is gone.
pub(crate) fn cargo(world: &World, vehicle: Entity) -> Option<Vec<Option<Held>>> {
    world.get::<Cargo>(vehicle).map(|cargo| cargo.0.clone())
}

/// Makes `slots` what `vehicle` carries: those who have it open see each changed slot, and its
/// stored compound keeps them.
pub(crate) fn load_cargo(world: &mut World, vehicle: Entity, slots: Vec<Option<Held>>) {
    let Some(before) = cargo(world, vehicle) else { return };
    let changed: Vec<usize> = (0..slots.len().min(before.len())).filter(|slot| before[*slot] != slots[*slot]).collect();
    if changed.is_empty() {
        return;
    }
    world.entity_mut(vehicle).insert(Cargo(slots.clone()));
    let viewers = containers::carried_viewers(world, vehicle);
    containers::tell_changed(world, viewers, &changed, &slots);
    let items: Vec<(u8, ItemStack)> = slots.iter().enumerate().filter_map(|(slot, held)| Some((slot as u8, held.as_ref()?.stack.clone()))).collect();
    entities::restore(world, vehicle, |compound, registry| sapling_world::chested_actor(compound, &items, registry).ok());
}

/// Opens the screen of what `vehicle` carries for a player; false for one that carries nothing.
pub(crate) fn open(world: &mut World, player: Entity, vehicle: Entity) -> bool {
    let Some((_, window)) = world.get::<Kind>(vehicle).and_then(|kind| hold(&kind.0)) else { return false };
    let (Some(slots), Some(id), Some(layer), Some(body)) = (cargo(world, vehicle), world.get::<NetId>(vehicle).map(|id| id.0), world.get::<InLayer>(vehicle).map(|layer| layer.0), world.get::<Body>(vehicle)) else { return false };
    let pos = body.at.map(|c| c.floor() as i32);
    containers::show_carried(world, player, layer, pos, (vehicle, id), window, &slots)
}

/// Tosses out everything `vehicle` carries, as it breaks.
pub(super) fn spill(world: &mut World, vehicle: Entity) {
    let (Some(slots), Some(layer), Some(body)) = (cargo(world, vehicle), world.get::<InLayer>(vehicle).map(|layer| layer.0), world.get::<Body>(vehicle).copied()) else { return };
    world.entity_mut(vehicle).remove::<Cargo>();
    for held in slots.into_iter().flatten() {
        drops::let_go(world, layer, held.stack, body.at);
    }
}

/// Lets each hopper minecart take up the first item lying in its reach that fits, when due.
pub(super) fn take_up(world: &mut World) {
    if world.resource::<Tick>().0 % HOPPER_EVERY != 0 {
        return;
    }
    let carts: Vec<(Entity, Entity, Body)> = world.query::<(Entity, &Kind, &InLayer, &Body)>().iter(world).filter(|(_, kind, ..)| kind.0 == HOPPER).map(|(cart, _, layer, body)| (cart, layer.0, *body)).collect();
    for (cart, layer, body) in carts {
        let Some(mut slots) = cargo(world, cart) else { continue };
        let [x, y, z] = body.at;
        let (min, max) = ([x - HOPPER_REACH, y - HOPPER_REACH, z - HOPPER_REACH], [x + HOPPER_REACH, y + body.size[1], z + HOPPER_REACH]);
        for (item, stack) in drops::lying_in(world, layer, min, max) {
            let taken = hoppers::insert(world, &mut slots, &stack, |_| true);
            if taken > 0 {
                drops::take(world, item, taken);
                load_cargo(world, cart, slots);
                break;
            }
        }
    }
}
