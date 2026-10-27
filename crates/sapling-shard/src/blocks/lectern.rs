//! Lecterns, as BDS ran one (research/captures/bds-books.txt): a click holding a book and quill or
//! a written book puts it on an empty lectern, heard as `item.book.put` at the block's corner, and
//! the entity keeps it (`book`, `hasBook`, `page` 0, `totalPages`); a click on one with a book
//! opens its screen. LecternUpdate turns its page. A comparator reads it.
// TODO: the book is not taken back from the screen (not drilled); broken, the lectern leaves the
// book by its name only (entities.rs); the `powered_bit` pulse BDS gave a turn to page 0 is not.

use acacia_proto::nbt::Value;
use acacia_proto::packets::LecternUpdate;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, GameMode};
use sapling_world::ItemStack;

use super::{fields, keep_entity, level, send};
use crate::components::{InLayer, Mode, PacketQueues};
use crate::inventory::{self, Inventory, Place};

const LECTERN: &str = "minecraft:lectern";
const ENTITY: &str = "Lectern";
pub(super) const BOOK: &str = "book";
const BOOKS: [&str; 2] = ["minecraft:writable_book", "minecraft:written_book"];

fn is_lectern(world: &World, layer: Entity, pos: BlockPos) -> bool {
    send::block_at(world, layer, pos).is_some_and(|block| block.name == LECTERN)
}

fn int(world: &World, layer: Entity, pos: BlockPos, key: &str) -> Option<i32> {
    match fields::get(world, layer, pos, key)? {
        Value::Int(value) => Some(value),
        _ => None,
    }
}

/// The page shown and the pages of the book on the lectern at `pos`, if one is on it.
fn opened_at(world: &World, layer: Entity, pos: BlockPos) -> Option<(i32, i32)> {
    (fields::byte(world, layer, pos, "hasBook")? != 0).then_some(())?;
    Some((int(world, layer, pos, "page").unwrap_or(0), int(world, layer, pos, "totalPages").unwrap_or(0)))
}

/// What a comparator reads of the block at `pos`: `None` for what is no lectern. BDS gave 15 for a
/// book at page 0 and at page 1 of 3, read afresh, where the Java game reads the page.
pub(super) fn signal(world: &World, layer: Entity, pos: BlockPos) -> Option<u8> {
    is_lectern(world, layer, pos).then_some(())?;
    Some(if opened_at(world, layer, pos).is_some() { 15 } else { 0 })
}

/// Sets the lectern's entity fields `set` at once and shows it.
fn keep(world: &mut World, layer: Entity, pos: BlockPos, set: Vec<(&str, Value)>) {
    let Some(level) = level(world, layer) else { return };
    let entity = level.block_entity(pos).map_or_else(|| sapling_world::new_block_entity(ENTITY, pos), |entity| entity.nbt.clone());
    let Value::Compound(mut fields) = entity else { return };
    for (key, value) in set {
        fields.retain(|(name, _)| name != key);
        fields.push((key.into(), value));
    }
    fields.sort_by(|a, b| a.0.cmp(&b.0));
    keep_entity(world, layer, pos, &Value::Compound(fields));
    super::container_changed(world, layer, pos);
}

/// A click on the block `name` at `pos`: true when it put a book on a lectern or there was
/// nothing to do; false lets a lectern with a book open its screen, and anything else go on.
pub(super) fn use_lectern(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, name: &str) -> bool {
    if name != LECTERN || opened_at(world, layer, pos).is_some() {
        return false;
    }
    let held = world.get::<Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref()).map(|held| held.stack.clone());
    let Some(book) = held.filter(|stack| BOOKS.contains(&stack.name.as_str())) else { return true };
    let pages = match book.tag.as_ref().and_then(|tag| tag.get("pages")) {
        Some(Value::List(list)) => list.items.len() as i32,
        _ => 0,
    };
    let Some(kept) = level(world, layer).map(|level| Value::Compound(ItemStack { count: 1, ..book }.fields(level.registry()))) else { return true };
    let creative = world.get::<Mode>(player).is_some_and(|mode| mode.0 == GameMode::Creative);
    if let Some(mut inventory) = world.get_mut::<Inventory>(player).filter(|_| !creative) {
        let slot = Place::Main(inventory.selected.into());
        let _ = inventory.take(slot, 1);
    }
    inventory::show_windows(world, player, true);
    let [x, y, z] = pos.map(|c| c as f32);
    send::to_viewers(world, layer, pos, &[send::noise("item.book.put", acacia_proto::types::Vec3f { x, y, z }, -1)]);
    keep(world, layer, pos, vec![(BOOK, kept), ("hasBook", Value::Byte(1)), ("page", Value::Int(0)), ("totalPages", Value::Int(pages))]);
    true
}

/// Turns the pages players asked to on lecterns that hold a book.
pub(crate) fn turn_pages(world: &mut World) {
    for (player, update) in world.resource_mut::<PacketQueues>().take::<LecternUpdate>() {
        let pos = [update.position.x, update.position.y, update.position.z];
        let Some(layer) = world.get::<InLayer>(player).map(|layer| layer.0) else { continue };
        let Some((_, pages)) = opened_at(world, layer, pos).filter(|_| is_lectern(world, layer, pos) && super::reaches(world, player, pos)) else { continue };
        if i32::from(update.page) < pages.max(1) {
            keep(world, layer, pos, vec![("page", Value::Int(update.page.into()))]);
        }
    }
}
