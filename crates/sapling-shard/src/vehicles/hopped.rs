//! Hoppers and the vehicles that carry goods (bds-ffcart.txt): a hopper fills a chest or hopper minecart
//! in the cell it faces and empties one in the cell over it, as it would a chest there; a hopper minecart
//! empties the container over it as a hopper does, then waits as after taking up an item.

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};

use super::cargo::{Cargo, cargo, load_cargo};
use crate::body::Body;
use crate::components::{InLayer, NetId};
use crate::inventory::Held;
use crate::{containers, hoppers};

/// The oldest vehicle with goods whose box reaches into the cell `pos` of `layer`.
fn carrier_in(world: &mut World, layer: Entity, [x, y, z]: BlockPos) -> Option<Entity> {
    let (min, max) = ([x as f32, y as f32, z as f32], [x as f32 + 1.0, y as f32 + 1.0, z as f32 + 1.0]);
    let mut carriers = world.query_filtered::<(Entity, &InLayer, &Body, &NetId), With<Cargo>>();
    let reaches = |body: &Body| {
        let (low, high) = body.bounds();
        (0..3).all(|axis| low[axis] < max[axis] && high[axis] > min[axis])
    };
    carriers.iter(world).filter(|(_, on, body, _)| on.0 == layer && reaches(body)).min_by_key(|(.., id)| id.0).map(|(vehicle, ..)| vehicle)
}

fn take_one(slot: &mut Option<Held>) {
    match slot.as_mut().filter(|held| held.stack.count > 1) {
        Some(held) => held.stack.count -= 1,
        None => *slot = None,
    }
}

/// One item of the first of a hopper's `slots` that has room in the vehicle in the cell `target`, put
/// into it; `None` where no such vehicle is there.
pub(crate) fn hopper_push(world: &mut World, layer: Entity, target: BlockPos, slots: &mut [Option<Held>]) -> Option<bool> {
    let vehicle = carrier_in(world, layer, target)?;
    let mut goods = cargo(world, vehicle)?;
    for slot in 0..slots.len() {
        let Some(one) = slots[slot].as_ref().map(|held| sapling_world::ItemStack { count: 1, ..held.stack.clone() }) else { continue };
        if hoppers::insert(world, &mut goods, &one, |_| true) > 0 {
            take_one(&mut slots[slot]);
            load_cargo(world, vehicle, goods);
            return Some(true);
        }
    }
    Some(false)
}

/// One item of the vehicle in the cell `source` taken into a hopper's `slots`, from its first slot
/// that has one that fits; `None` where no such vehicle is there.
pub(crate) fn hopper_pull(world: &mut World, layer: Entity, source: BlockPos, slots: &mut [Option<Held>]) -> Option<bool> {
    let vehicle = carrier_in(world, layer, source)?;
    let mut goods = cargo(world, vehicle)?;
    for slot in 0..goods.len() {
        let Some(one) = goods[slot].as_ref().map(|held| sapling_world::ItemStack { count: 1, ..held.stack.clone() }) else { continue };
        if hoppers::insert(world, slots, &one, |_| true) > 0 {
            take_one(&mut goods[slot]);
            load_cargo(world, vehicle, goods);
            return Some(true);
        }
    }
    Some(false)
}

/// The hopper minecart `cart` with its box's bottom at `feet` takes one item from the container over it,
/// as a hopper in its cell would; `None` where there is no container there.
pub(super) fn pull_from_above(world: &mut World, layer: Entity, cart: Entity, feet: [f32; 3]) -> Option<bool> {
    let cell = [feet[0].floor() as i32, (feet[1] + 0.5).floor() as i32, feet[2].floor() as i32];
    let above = containers::kind_at(world, layer, Face::Up.neighbour(cell)).filter(|kind| kind.holds_items())?;
    let mut goods = cargo(world, cart)?;
    let pulled = hoppers::pull_from_above(world, layer, cell, &mut goods, above);
    if pulled {
        load_cargo(world, cart, goods);
    }
    Some(pulled)
}

/// What a comparator reads off `vehicle`'s goods, as off a container's slots (bds-ffcart.txt: a detector
/// rail under a hopper minecart read 3, 6, 9 for one, two, three stacks).
pub(crate) fn signal(world: &World, vehicle: Entity) -> Option<u8> {
    world.get::<Cargo>(vehicle)?;
    let goods = cargo(world, vehicle)?;
    Some(crate::blocks::fullness(goods.iter().flatten().map(|held| (held.stack.count, sapling_data::max_stack(&held.stack.name))), goods.len()))
}
