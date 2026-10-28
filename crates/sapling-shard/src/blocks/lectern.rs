//! Lecterns, as BDS ran one (research/captures/bds-books.txt): a click holding a book and quill or
//! a written book puts it on an empty lectern, heard as `item.book.put` at the block's corner, and
//! the entity keeps it (`book`, `hasBook`, `page` 0, `totalPages`); a click on one with a book
//! opens its screen. LecternUpdate turns its page. A comparator reads it.
//! A turn to page 0 powers it a tick later for two ticks, as BDS did; turns to other pages showed no
//! power (bds-books.txt). Broken, it leaves the book whole (entities.rs).

use acacia_proto::nbt::Value;
use acacia_proto::packets::LecternUpdate;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, GameMode};
use sapling_world::ItemStack;

use super::queued::{self, Job};
use super::{fields, keep_entity, level, send};
use crate::components::{InLayer, Mode, PacketQueues};
use crate::inventory::{self, Inventory, Place};

const LECTERN: &str = "minecraft:lectern";
const ENTITY: &str = "Lectern";
pub(super) const BOOK: &str = "book";
const BOOKS: [&str; 2] = ["minecraft:writable_book", "minecraft:written_book"];
/// Ticks a turned lectern stays powered.
const PULSE: u64 = 2;

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

/// A hit on the block at `pos`: a lectern that holds a book lets it go, as the Bedrock client's
/// Take Book does by a hit (dragonfly's and PocketMine's `Lectern` drop it on a punch; not
/// drilled). False for anything else.
pub(super) fn hit(world: &mut World, layer: Entity, pos: BlockPos) -> bool {
    if !is_lectern(world, layer, pos) || opened_at(world, layer, pos).is_none() {
        return false;
    }
    let Some(book) = fields::get(world, layer, pos, BOOK).and_then(|book| ItemStack::parse(&book)) else { return false };
    let Some(Value::Compound(mut kept)) = level(world, layer).and_then(|level| level.block_entity(pos)).map(|entity| entity.nbt.clone()) else { return false };
    kept.retain(|(key, _)| !matches!(key.as_str(), BOOK | "hasBook" | "page" | "totalPages"));
    keep_entity(world, layer, pos, &Value::Compound(kept));
    super::container_changed(world, layer, pos);
    crate::drops::spill(world, layer, book, sapling_api::Face::Up.neighbour(pos));
    true
}

/// Turns the pages players asked to on lecterns that hold a book.
pub(crate) fn turn_pages(world: &mut World) {
    for (player, update) in world.resource_mut::<PacketQueues>().take::<LecternUpdate>() {
        let pos = [update.position.x, update.position.y, update.position.z];
        let Some(layer) = world.get::<InLayer>(player).map(|layer| layer.0) else { continue };
        let Some((from, pages)) = opened_at(world, layer, pos).filter(|_| is_lectern(world, layer, pos) && super::reaches(world, player, pos)) else { continue };
        let to = i32::from(update.page);
        if to >= pages.max(1) {
            continue;
        }
        let Some(page) = crate::hand_events::page_turned(world, player, pos, from, to).filter(|page| (0..pages.max(1)).contains(page)) else {
            super::send::resync(world, player, &[pos]);
            continue;
        };
        keep(world, layer, pos, vec![("page", Value::Int(page))]);
        if page == 0 {
            queued::cancel(world, layer, pos, Job::Lectern);
            queued::after(world, layer, pos, 1, Job::Lectern);
        }
    }
}

/// The page-turn pulse: an unpowered lectern is powered for [`PULSE`] ticks, a powered one goes off.
pub(super) fn pulse(world: &mut World, layer: Entity, [x, y, z]: BlockPos) {
    let Some(level) = level(world, layer).filter(|level| level.in_height(y)) else { return };
    let id = level.block(x, y, z);
    let Some(state) = level.registry().get(id).filter(|state| state.name == LECTERN) else { return };
    let on = !state.properties.split(',').any(|pair| pair == "powered_bit=1");
    let Some(set) = super::with_property(level.registry(), id, "powered_bit", if on { "1" } else { "0" }) else { return };
    send::set(world, layer, [x, y, z], set);
    if on {
        queued::after(world, layer, [x, y, z], PULSE, Job::Lectern);
    }
}
