//! What chest boats and chest and hopper minecarts carry (docs/research/vehicles.md "Cargo"):
//! slots kept on the vehicle and in its stored `ChestItems`, shown on a screen naming the
//! vehicle, spilled when it breaks; a hopper minecart takes up the items it rolls over.
// TODO: hoppers neither fill nor empty vehicles, and comparators do not read them.

use acacia_proto::types::WindowType;
use bevy_ecs::prelude::*;
use sapling_world::ItemStack;

use crate::body::Body;
use crate::components::{InLayer, NetId};
use crate::entities::{Kind, Stored};
use crate::inventory::Held;
use crate::{containers, drops, entities, hoppers};

const HOPPER: &str = "minecraft:hopper_minecart";
/// A hopper minecart that took an item waits this many ticks; its box reaches this much wider.
const HOPPER_EVERY: u64 = 4;
const HOPPER_REACH: f32 = 0.25;

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

/// The slots of `vehicle`, or of a mount's inventory (riding/packs.rs); `None` for one that
/// carries nothing or is gone.
pub(crate) fn cargo(world: &World, vehicle: Entity) -> Option<Vec<Option<Held>>> {
    world.get::<Cargo>(vehicle).map(|cargo| cargo.0.clone()).or_else(|| crate::riding::pack_slots(world, vehicle))
}

/// Makes `slots` what `vehicle` carries: those who have it open see each changed slot, and its
/// stored compound keeps them.
pub(crate) fn load_cargo(world: &mut World, vehicle: Entity, slots: Vec<Option<Held>>) {
    if world.get::<Cargo>(vehicle).is_none() {
        return crate::riding::load_pack(world, vehicle, slots);
    }
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
    show(world, player, vehicle, window)
}

/// Opens it for its rider's inventory key: a plain `Container` screen (bds-t2eboats.txt: a chest
/// boat's, at the boat's cell).
pub(crate) fn open_seated(world: &mut World, player: Entity, vehicle: Entity) -> bool {
    world.get::<Kind>(vehicle).and_then(|kind| hold(&kind.0)).is_some() && show(world, player, vehicle, WindowType::Container)
}

fn show(world: &mut World, player: Entity, vehicle: Entity, window: WindowType) -> bool {
    let (Some(slots), Some(id), Some(layer), Some(body)) = (cargo(world, vehicle), world.get::<NetId>(vehicle).map(|id| id.0), world.get::<InLayer>(vehicle).map(|layer| layer.0), world.get::<Body>(vehicle)) else { return false };
    let pos = body.at.map(|c| c.floor() as i32);
    containers::show_carried(world, player, layer, pos, (vehicle, id), window, &slots, None)
}

/// Tosses out everything `vehicle` carries, as it breaks.
pub(super) fn spill(world: &mut World, vehicle: Entity) {
    let (Some(slots), Some(layer), Some(body)) = (cargo(world, vehicle), world.get::<InLayer>(vehicle).map(|layer| layer.0), world.get::<Body>(vehicle).copied()) else { return };
    world.entity_mut(vehicle).remove::<Cargo>();
    for held in slots.into_iter().flatten() {
        drops::let_go(world, layer, held.stack, body.at);
    }
}

/// The ticks a hopper minecart has to wait, and the cell it waits in: moving on ends the wait.
#[derive(Component, Default)]
struct HopperWait {
    left: u64,
    cell: [i32; 3],
}

/// Where a hopper minecart with the box `low..high` looks for items, in order (BDS
/// `HopperSystem` 148489a30 and `Hopper::_tryAddItemsFromPos` 1429cdfe0): a block wide around its
/// box's middle, from the middle up, a block at a time; then its box a quarter wider each way.
fn reaches(low: [f32; 3], high: [f32; 3]) -> [([f32; 3], [f32; 3]); 3] {
    let middle = [0, 1, 2].map(|axis| (low[axis] + high[axis]) / 2.0);
    let around = |up: f32| ([middle[0] - 0.5, middle[1] + up, middle[2] - 0.5], [middle[0] + 0.5, middle[1] + up + 1.0, middle[2] + 0.5]);
    let wider = ([low[0] - HOPPER_REACH, low[1], low[2] - HOPPER_REACH], [high[0] + HOPPER_REACH, high[1], high[2] + HOPPER_REACH]);
    [around(0.0), around(1.0), wider]
}

/// The items whose box overlaps `min..max`, oldest first.
fn items_touching(world: &mut World, layer: Entity, min: [f32; 3], max: [f32; 3]) -> Vec<(Entity, ItemStack)> {
    let mut lying = world.query::<(Entity, &drops::Dropped, &Body, &InLayer, &NetId)>();
    let overlaps = |body: &Body| {
        let (low, high) = body.bounds();
        (0..3).all(|axis| low[axis] < max[axis] && high[axis] > min[axis])
    };
    let mut found: Vec<_> = lying.iter(world).filter(|(_, _, body, on, _)| on.0 == layer && overlaps(body)).map(|(item, dropped, _, _, id)| (id.0, item, dropped.stack().clone())).collect();
    found.sort_unstable_by_key(|(id, ..)| *id);
    found.into_iter().map(|(_, item, stack)| (item, stack)).collect()
}

/// Lets each hopper minecart not waiting take up the first item in its reach that fits, and wait
/// after it took one; one on a powered activator rail is locked (research/captures/bds-e2rails.txt,
/// bds-i2reach.txt).
pub(super) fn take_up(world: &mut World) {
    let carts: Vec<(Entity, Entity, Body)> = world.query::<(Entity, &Kind, &InLayer, &Body)>().iter(world).filter(|(_, kind, ..)| kind.0 == HOPPER).map(|(cart, _, layer, body)| (cart, layer.0, *body)).collect();
    for (cart, layer, body) in carts {
        let (low, high) = body.bounds();
        let cell = [0, 1, 2].map(|axis| ((low[axis] + high[axis]) / 2.0).floor() as i32);
        let wait = world.get::<HopperWait>(cart).map_or(0, |wait| if wait.cell == cell { wait.left } else { 0 });
        world.entity_mut(cart).insert(HopperWait { left: wait.saturating_sub(1), cell });
        let Some(level) = world.get::<crate::components::Layer>(layer).map(|layer| &layer.world) else { continue };
        if wait > 0 || super::rails::rail_under(level, body.at).is_some_and(|rail| rail.kind == super::rails::RailKind::Activator && rail.on) {
            continue;
        }
        let Some(mut slots) = cargo(world, cart) else { continue };
        'reach: for (min, max) in reaches(low, high) {
            for (item, stack) in items_touching(world, layer, min, max) {
                let taken = hoppers::insert(world, &mut slots, &stack, |_| true);
                if taken > 0 {
                    drops::take(world, item, taken);
                    load_cargo(world, cart, slots);
                    world.entity_mut(cart).insert(HopperWait { left: HOPPER_EVERY, cell });
                    break 'reach;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::reaches;

    /// research/captures/bds-i2reach.txt: a cart at x 0.5 took items lying 0.55 to 0.8 east and
    /// up to 2 blocks over its rail, and none 3 over.
    #[test]
    fn a_hopper_minecart_reaches_a_quarter_past_its_box_and_two_blocks_up_from_its_middle() {
        let item = |x: f32, y: f32| ([x - 0.125, y, -0.125], [x + 0.125, y + 0.25, 0.125]);
        let takes = |(low, high): ([f32; 3], [f32; 3])| reaches([0.01, 0.0, -0.49], [0.99, 0.7, 0.49]).iter().any(|(min, max)| (0..3).all(|axis| low[axis] < max[axis] && high[axis] > min[axis]));
        assert!([1.05, 1.15, 1.3].into_iter().all(|x| takes(item(x, 0.0))));
        assert!(!takes(item(1.4, 0.0)));
        assert!([1.0, 1.5, 2.0].into_iter().all(|y| takes(item(0.95, y))));
        assert!(!takes(item(0.95, 3.0)));
    }
}
