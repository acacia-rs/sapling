//! Chiseled bookshelves, as BDS ran one (research/captures/bds-bookshelves.txt,
//! docs/research/block-entities.md "Chiseled bookshelves"): a click on the front picks one of six
//! slots by where it lands (three across, two down); a book goes into an empty slot, and any
//! click on a full one takes its book. `books_stored` has a bit a slot; the entity keeps all six
//! slots (`Items`, empty ones nameless) and `LastInteractedSlot`, the slot plus one, which a
//! comparator reads. With no book left the entity is bare again, but the comparator still read
//! the slot last used ([`LastSlots`]). Broken, it leaves its books.

use std::collections::HashMap;

use acacia_proto::nbt::{List, Value};
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};
use sapling_world::{ItemStack, new_block_entity};

use super::send::keep_entity;
use super::{level, send, with_property};
use crate::inventory::{self, Inventory};

const SHELF: &str = "minecraft:chiseled_bookshelf";
const ENTITY: &str = "ChiseledBookshelf";
const SLOTS: usize = 6;
const LAST: &str = "LastInteractedSlot";
/// The items a slot takes (the `bookshelf_books` tag).
const BOOKS: &[&str] = &["minecraft:book", "minecraft:writable_book", "minecraft:written_book", "minecraft:enchanted_book", "minecraft:knowledge_book"];
/// `TAG_Compound`, the element tag of the list of slots.
const COMPOUND: u8 = 10;

/// The slot plus one last used on each emptied shelf, which its bare entity does not keep.
#[derive(Resource, Default)]
struct LastSlots(HashMap<(Entity, BlockPos), u8>);

/// The face a shelf's `direction` (south, west, north, east) shows its books on.
fn front(direction: &str) -> Option<Face> {
    Some(match direction {
        "0" => Face::South,
        "1" => Face::West,
        "2" => Face::North,
        "3" => Face::East,
        _ => return None,
    })
}

/// The slot a click at `at` (within the block) on the front `face` lands on, as the Java game's
/// `ChiseledBookShelfBlock.getHitSlot`, which BDS matched at the face's middle.
fn slot_hit(face: Face, at: [f32; 3]) -> usize {
    let across = match face {
        Face::North => 1.0 - at[0],
        Face::South => at[0],
        Face::West => at[2],
        _ => 1.0 - at[2],
    };
    let column = if across < 0.375 { 0 } else if across < 0.6875 { 1 } else { 2 };
    column + if at[1] >= 0.5 { 0 } else { 3 }
}

/// The shelf's six slots from its entity.
fn shelved(entity: Option<&Value>) -> [Option<ItemStack>; SLOTS] {
    let mut slots: [Option<ItemStack>; SLOTS] = Default::default();
    if let Some(Value::List(items)) = entity.and_then(|entity| entity.get("Items")) {
        for (slot, item) in slots.iter_mut().zip(&items.items) {
            *slot = ItemStack::parse(item);
        }
    }
    slots
}

/// The books a broken shelf leaves (entities.rs).
pub(super) fn books(kept: &Value) -> Vec<ItemStack> {
    match kept.get("id") {
        Some(Value::String(id)) if id.as_str() == ENTITY => shelved(Some(kept)).into_iter().flatten().collect(),
        _ => Vec::new(),
    }
}

/// What a comparator reads of the block at `pos`: `None` for what is no chiseled bookshelf.
pub(super) fn signal(world: &World, layer: Entity, pos: BlockPos) -> Option<u8> {
    let level = level(world, layer)?;
    level.registry().get(level.block(pos[0], pos[1], pos[2])).filter(|state| state.name == SHELF)?;
    match level.block_entity(pos).and_then(|entity| entity.nbt.get(LAST).cloned()) {
        Some(Value::Int(last)) => u8::try_from(last).ok(),
        _ => Some(world.get_resource::<LastSlots>().and_then(|slots| slots.0.get(&(layer, pos)).copied()).unwrap_or(0)),
    }
}

/// Writes the shelf's `slots`, its `books_stored` and the slot last used, with the sound BDS made
/// first, named after the shelf as it was.
fn store(world: &mut World, layer: Entity, pos: BlockPos, slots: &[Option<ItemStack>; SLOTS], last: usize, sound: &str) {
    let Some(level) = level(world, layer) else { return };
    let (registry, before) = (level.registry(), level.block(pos[0], pos[1], pos[2]));
    let heard = send::sound(sound, registry, pos, before);
    let bits: u8 = slots.iter().enumerate().filter(|(_, slot)| slot.is_some()).map(|(index, _)| 1 << index).sum();
    let after = with_property(registry, before, "books_stored", &bits.to_string());
    let mut entity = new_block_entity(ENTITY, pos);
    if let (Value::Compound(fields), true) = (&mut entity, bits > 0) {
        let empty = ItemStack { name: String::new(), count: 0, damage: 0, block: None, tag: None };
        let item = |stack: &ItemStack| {
            let mut fields = stack.fields(registry);
            fields.push(("WasPickedUp".into(), Value::Byte(0)));
            fields.sort_by(|a, b| a.0.cmp(&b.0));
            Value::Compound(fields)
        };
        let items = slots.iter().map(|slot| item(slot.as_ref().unwrap_or(&empty))).collect();
        fields.extend([("Items".into(), Value::List(List { tag: COMPOUND, items })), (LAST.into(), Value::Int(last as i32 + 1))]);
        fields.sort_by(|a, b| a.0.cmp(&b.0));
    }
    let mut emptied = world.get_resource_or_insert_with(LastSlots::default);
    match bits {
        0 => emptied.0.insert((layer, pos), last as u8 + 1),
        _ => emptied.0.remove(&(layer, pos)),
    };
    send::to_viewers(world, layer, pos, &[heard]);
    if let Some(after) = after {
        send::set(world, layer, pos, after);
    }
    keep_entity(world, layer, pos, &entity);
}

/// A click on `face` at `at` (within the block) of the block at `pos`: false when it is no
/// shelf's front, or the slot is empty and the hand holds no book.
pub(super) fn use_on(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, face: Face, at: [f32; 3], creative: bool) -> bool {
    let Some(level) = level(world, layer) else { return false };
    let Some(state) = level.registry().get(level.block(pos[0], pos[1], pos[2])).filter(|state| state.name == SHELF) else { return false };
    if state.property("direction").and_then(front) != Some(face) {
        return false;
    }
    let mut slots = shelved(level.block_entity(pos).map(|entity| &entity.nbt));
    let slot = slot_hit(face, at);
    if let Some(book) = slots[slot].take() {
        store(world, layer, pos, &slots, slot, "pickup");
        if inventory::give_and_pop(world, player, &book).is_some_and(|left| left > 0) {
            crate::drops::spill(world, layer, ItemStack { count: 1, ..book }, pos);
        }
        return true;
    }
    let held = world.get::<Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref()).map(|held| held.stack.clone());
    let Some(book) = held.filter(|held| BOOKS.contains(&held.name.as_str())) else { return false };
    slots[slot] = Some(ItemStack { count: 1, ..book });
    store(world, layer, pos, &slots, slot, "insert");
    if !creative {
        inventory::use_up_held(world, player);
    }
    true
}

/// Leaves the books of the shelf at `pos`, broken `by_creative`, on the ground; call before it
/// goes. Other breaks leave them with their loot (entities.rs).
pub(super) fn spill(world: &mut World, layer: Entity, pos: BlockPos, by_creative: bool) {
    if let Some(mut emptied) = world.get_resource_mut::<LastSlots>() {
        emptied.0.remove(&(layer, pos));
    }
    let Some(kept) = level(world, layer).and_then(|level| level.block_entity(pos)).map(|entity| books(&entity.nbt)).filter(|_| by_creative) else { return };
    kept.into_iter().for_each(|book| crate::drops::spill(world, layer, book, pos));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_click_picks_a_slot_by_where_it_lands_on_the_front() {
        // bds-bookshelves.txt: the middle of a west front is slot 1 (`LastInteractedSlot` 2).
        assert_eq!(slot_hit(Face::West, [0.0, 0.5, 0.5]), 1);
        assert_eq!([slot_hit(Face::North, [0.9, 0.9, 0.0]), slot_hit(Face::North, [0.1, 0.1, 0.0])], [0, 5]);
        assert_eq!([slot_hit(Face::East, [1.0, 0.2, 0.9]), slot_hit(Face::South, [0.5, 0.2, 1.0])], [3, 4]);
    }
}
