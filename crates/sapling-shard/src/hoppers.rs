//! Hoppers moving items, as BDS's `HopperBlockActor::_tick` and `Hopper::_tryMoveItems` do
//! (docs/research/hoppers.md). A ticking hopper (ticking.rs) whose cooldown is over and that is
//! not powered pushes one item into the container it faces, then pulls one from the container
//! above or, when there is none, takes an item lying on it, whole if it fits. A move sets the
//! cooldown; a hopper that moved nothing tries again the next tick. Their slots are a
//! container's (containers.rs).
// TODO: BDS first ticks the hoppers below a hopper, so a chain moves an item along in one tick;
// composters are not fed or emptied.

use std::collections::HashMap;

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};
use sapling_world::ItemStack;

use crate::components::{Layer, Tick};
use crate::containers::{self, Kind};
use crate::inventory::{Held, stacks_with};
use crate::{drops, furnaces};

/// Ticks from one move to the next: BDS was recorded moving an item every 400 ms
/// (`examples/bds_container_probe.rs`); its code waits 4 hopper ticks, so those are 2 game ticks.
const MOVE_TICKS: u32 = 8;
const FACING: &str = "facing_direction";
const POWERED: &str = "toggle_bit";
const COOLDOWN: &str = "TransferCooldown";
/// Lying items are taken from this high above the hopper's cell, to this high.
const PICKUP: [f32; 2] = [0.625, 2.0];

struct Hopper {
    cooldown: u32,
    last_tick: u64,
}

/// The hoppers that tick, by layer and block.
#[derive(Resource, Default)]
pub(crate) struct Hoppers(HashMap<(Entity, BlockPos), Hopper>);

fn hoppers(world: &mut World) -> Mut<'_, Hoppers> {
    world.get_resource_or_insert_with(Hoppers::default)
}

/// Starts ticking the hopper at `pos`, holding its slots, with the cooldown its block entity kept.
pub(crate) fn watch(world: &mut World, layer: Entity, pos: BlockPos) {
    if hoppers(world).0.contains_key(&(layer, pos)) {
        return;
    }
    let kept = world
        .get::<Layer>(layer)
        .and_then(|level| level.world.block_entity(pos))
        .and_then(|entity| match entity.nbt.get(COOLDOWN) {
            Some(Value::Int(ticks)) => Some((*ticks).max(0) as u32),
            _ => None,
        });
    if containers::pin(world, layer, pos) {
        hoppers(world).0.insert(
            (layer, pos),
            Hopper {
                cooldown: kept.unwrap_or(0),
                last_tick: 0,
            },
        );
    }
}

pub(crate) fn watched(world: &World) -> Vec<(Entity, BlockPos)> {
    world
        .get_resource::<Hoppers>()
        .map(|hoppers| hoppers.0.keys().copied().collect())
        .unwrap_or_default()
}

/// Stops ticking the hopper at `pos`, its cooldown kept in its block entity.
pub(crate) fn forget(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(hopper) = world
        .get_resource_mut::<Hoppers>()
        .and_then(|mut hoppers| hoppers.0.remove(&(layer, pos)))
    else {
        return;
    };
    let entity = world
        .get::<Layer>(layer)
        .and_then(|level| level.world.block_entity(pos))
        .map(|entity| entity.nbt.clone());
    if let Some(Value::Compound(mut fields)) = entity {
        fields.retain(|(key, _)| key != COOLDOWN);
        fields.push((COOLDOWN.into(), Value::Int(hopper.cooldown as i32)));
        if let Some(mut level) = world.get_mut::<Layer>(layer) {
            level.world.set_block_entity(pos, &Value::Compound(fields));
        }
    }
    containers::unpin(world, layer, pos);
}

/// The way the hopper at `pos` faces and whether it is powered; `None` once it is no hopper.
fn state(world: &World, layer: Entity, pos: BlockPos) -> Option<(Face, bool)> {
    let level = &world.get::<Layer>(layer)?.world;
    let state = level.registry().get(level.block(pos[0], pos[1], pos[2]))?;
    let number = |key: &str| {
        state
            .properties
            .split(',')
            .filter_map(|pair| pair.split_once('='))
            .find(|(name, _)| *name == key)?
            .1
            .parse::<u32>()
            .ok()
    };
    Some((
        Face::from_index(number(FACING)?)?,
        number(POWERED) == Some(1),
    ))
}

/// Puts as many of `item` as fit into the first slot of `slots` that `accepts` it, empty or
/// holding its kind; returns how many went in.
fn insert(
    world: &mut World,
    slots: &mut [Option<Held>],
    item: &ItemStack,
    accepts: impl Fn(usize) -> bool,
) -> u8 {
    let limit = sapling_data::max_stack(&item.name);
    for slot in (0..slots.len()).filter(|slot| accepts(*slot)) {
        match &mut slots[slot] {
            None => {
                slots[slot] = Some(containers::new_held(world, item.clone()));
                return item.count;
            }
            Some(held) if stacks_with(&held.stack, item) && held.stack.count < limit => {
                let fits = item.count.min(limit - held.stack.count);
                held.stack.count += fits;
                return fits;
            }
            Some(_) => {}
        }
    }
    0
}

fn take_one(slot: &mut Option<Held>) {
    if let Some(held) = slot.as_mut().filter(|held| held.stack.count > 1) {
        held.stack.count -= 1;
    } else {
        *slot = None;
    }
}

fn one(held: &Held) -> ItemStack {
    ItemStack {
        count: 1,
        ..held.stack.clone()
    }
}

/// Moves one item from the first slot that can give one into the container the hopper faces.
fn push(
    world: &mut World,
    layer: Entity,
    pos: BlockPos,
    slots: &mut [Option<Held>],
    facing: Face,
) -> bool {
    let target = facing.neighbour(pos);
    let Some(kind) = containers::kind_at(world, layer, target).filter(|kind| kind.holds_items())
    else {
        return false;
    };
    // The target's face that touches the hopper.
    let face = Face::from_index(facing as u32 ^ 1).unwrap_or(facing);
    let was_empty = containers::slots_at(world, layer, target)
        .is_some_and(|slots| slots.iter().all(Option::is_none));
    for slot in 0..slots.len() {
        let Some(item) = slots[slot].as_ref().map(one) else {
            continue;
        };
        let moved = containers::with_slots(world, layer, target, |world, into| {
            insert(world, into, &item, |at| {
                kind.takes_from_hopper(at, face, &item)
            }) > 0
        });
        if moved == Some(true) {
            take_one(&mut slots[slot]);
            if kind.is_hopper() && was_empty {
                delay_receiver(world, layer, target);
            }
            return true;
        }
    }
    false
}

/// A hopper that was empty and just got an item waits a whole cooldown before passing it on.
fn delay_receiver(world: &mut World, layer: Entity, pos: BlockPos) {
    let now = world.resource::<Tick>().0;
    if let Some(hopper) = hoppers(world).0.get_mut(&(layer, pos)) {
        hopper.cooldown = MOVE_TICKS + u32::from(hopper.last_tick < now);
    }
}

/// Moves one item into the hopper from the first slot of the container above that gives one.
fn pull_from_above(
    world: &mut World,
    layer: Entity,
    pos: BlockPos,
    slots: &mut [Option<Held>],
    above: Kind,
) -> bool {
    let source = Face::Up.neighbour(pos);
    let pulled = containers::with_slots(world, layer, source, |world, from| {
        for slot in 0..from.len() {
            let Some(item) = from[slot]
                .as_ref()
                .map(one)
                .filter(|item| above.gives_to_hopper(slot, Face::Down, item))
            else {
                continue;
            };
            if insert(world, slots, &item, |_| true) > 0 {
                take_one(&mut from[slot]);
                return Some((slot, item));
            }
        }
        None
    });
    let Some(Some((slot, item))) = pulled else {
        return false;
    };
    furnaces::taken_by_hopper(world, layer, source, slot, &item);
    true
}

/// Takes the first item lying on the hopper that fits, as much of it as fits in two slots.
fn pull_lying(
    world: &mut World,
    layer: Entity,
    [x, y, z]: BlockPos,
    slots: &mut [Option<Held>],
) -> bool {
    let (min, max) = (
        [x as f32, y as f32 + PICKUP[0], z as f32],
        [x as f32 + 1.0, y as f32 + PICKUP[1], z as f32 + 1.0],
    );
    for (item, stack) in drops::lying_in(world, layer, min, max) {
        let first = insert(world, slots, &stack, |_| true);
        let rest = ItemStack {
            count: stack.count - first,
            ..stack.clone()
        };
        let taken = first
            + if rest.count > 0 && first > 0 {
                insert(world, slots, &rest, |_| true)
            } else {
                0
            };
        if taken > 0 {
            drops::take(world, item, taken);
            return true;
        }
    }
    false
}

/// One move of the hopper at `pos`: a push, then a pull; true if either moved an item.
fn step(world: &mut World, layer: Entity, pos: BlockPos, facing: Face) -> bool {
    let Some(mut slots) = containers::slots_at(world, layer, pos) else {
        return false;
    };
    let pushed = slots.iter().any(Option::is_some) && push(world, layer, pos, &mut slots, facing);
    let pulled = match containers::kind_at(world, layer, Face::Up.neighbour(pos)) {
        Some(above) if above.holds_items() => pull_from_above(world, layer, pos, &mut slots, above),
        Some(_) => false,
        None => pull_lying(world, layer, pos, &mut slots),
    };
    containers::set_slots(world, layer, pos, slots);
    pushed || pulled
}

/// Lets every ticking hopper count down and, when it is due, move.
pub(crate) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let mut due: Vec<(Entity, BlockPos)> = Vec::new();
    for (key, hopper) in world
        .get_resource_mut::<Hoppers>()
        .into_iter()
        .flat_map(|hoppers| hoppers.into_inner().0.iter_mut())
    {
        hopper.last_tick = now;
        let old = hopper.cooldown;
        hopper.cooldown = old.saturating_sub(1);
        if old <= 1 {
            due.push(*key);
        }
    }
    due.sort_unstable_by_key(|(_, pos)| *pos);
    for (layer, pos) in due {
        let Some((facing, false)) = state(world, layer, pos) else {
            continue;
        };
        if step(world, layer, pos, facing) {
            if let Some(hopper) = hoppers(world).0.get_mut(&(layer, pos)) {
                hopper.cooldown = MOVE_TICKS;
            }
        }
    }
}
