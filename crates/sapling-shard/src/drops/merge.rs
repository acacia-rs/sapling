//! Lying items joining up as BDS's do (docs/research/items.md, merge): on every 25th tick of its
//! age an item meets the items of its kind whose boxes touch its own grown by half a block, and
//! of each two the bigger stack takes in the smaller whole, when the two fit in one stack.
// TODO: BDS removes the smaller from its viewers a tick after the merge (bds-tools.txt 16505/16547); sapling at once.

use acacia_proto::packets::{EntityEvent, EntityEventEventId};
use bevy_ecs::prelude::*;
use sapling_world::ItemStack;

use super::{Dropped, SIZE};
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId};
use crate::entities::Stored;
use crate::inventory::stacks_with;
use crate::{blocks, mobs};

const EVERY: u32 = 25;
const REACH: f32 = 0.5;

/// An item that may join another: its entity, wire id, place and what it holds.
#[derive(Clone)]
struct Lying {
    entity: Entity,
    id: u64,
    at: [f32; 3],
    stack: ItemStack,
    pickup_from: u64,
    age: u32,
}

fn lying(world: &World, entity: Entity) -> Option<Lying> {
    let item = world.get::<Dropped>(entity)?;
    Some(Lying { entity, id: world.get::<NetId>(entity)?.0, at: world.get::<Body>(entity)?.at, stack: item.stack.clone(), pickup_from: item.pickup_from, age: item.age })
}

/// The other items of `layer` of the kind `item` holds whose boxes touch its own grown by [`REACH`], oldest first.
fn touching(world: &mut World, layer: Entity, item: &Lying) -> Vec<Entity> {
    let near = |at: &[f32; 3]| (0..3).all(|axis| (at[axis] - item.at[axis]).abs() < SIZE + REACH);
    let mut items = world.query::<(Entity, &Dropped, &Body, &InLayer, &NetId)>();
    let mut found: Vec<_> = items.iter(world).filter(|(entity, other, body, in_layer, _)| *entity != item.entity && in_layer.0 == layer && near(&body.at) && stacks_with(&other.stack, &item.stack)).map(|(entity, .., id)| (id.0, entity)).collect();
    found.sort_unstable_by_key(|(id, _)| *id);
    found.into_iter().map(|(_, entity)| entity).collect()
}

/// On the item's 25th ticks (`age` the ticks before this one), the items it takes in or goes into.
pub(super) fn join(world: &mut World, entity: Entity, layer: Entity, age: u32) {
    if age == 0 || age % EVERY != 0 {
        return;
    }
    let Some(mut item) = lying(world, entity) else { return };
    for other in touching(world, layer, &item) {
        let Some(other) = lying(world, other) else { continue };
        // On a tie the other takes it in.
        let (big, small) = if other.stack.count < item.stack.count { (item.clone(), other) } else { (other, item.clone()) };
        let count = u16::from(big.stack.count) + u16::from(small.stack.count);
        let Ok(count) = u8::try_from(count) else { continue };
        if count > sapling_data::max_stack(&big.stack.name) {
            continue;
        }
        let grown = Lying { stack: ItemStack { count, ..big.stack.clone() }, pickup_from: big.pickup_from.max(small.pickup_from), age: big.age.min(small.age), ..big };
        keep(world, layer, &grown);
        mobs::remove(world, small.entity);
        let merged = sapling_net::encode(&EntityEvent { runtime_entity_id: grown.id, event_id: EntityEventEventId::ItemEntityMerge, data: i32::from(count), fire_at_position: None });
        blocks::to_viewers(world, layer, grown.at.map(|c| c.floor() as i32), &[merged]);
        if small.entity == entity {
            return;
        }
        item = grown;
    }
}

/// Keeps what `item` now holds, in the shard and for the world folder.
fn keep(world: &mut World, layer: Entity, item: &Lying) {
    if let Some(mut kept) = world.get_mut::<Dropped>(item.entity) {
        (kept.stack, kept.pickup_from, kept.age) = (item.stack.clone(), item.pickup_from, item.age);
    }
    let (Some(level), Some(stored)) = (world.get::<Layer>(layer), world.get::<Stored>(item.entity)) else { return };
    let Ok(grown) = sapling_world::lying_stack(&stored.compound, &item.stack, level.world.registry()) else { return };
    if let Some(mut stored) = world.get_mut::<Stored>(item.entity) {
        stored.compound = grown;
    }
}
