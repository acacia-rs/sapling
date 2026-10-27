//! Books and quills written in and signed with BookEdit, as BDS kept them (docs/research/item-uses.md,
//! "Books"): pages are `{photoname, text}` compounds in the stack's `pages` list; a ReplacePage past
//! the end pads with empty pages, an AddPage inserts, a DeletePage and a SwapPages touch only pages
//! there are. A Sign makes the stack a written book with the client's `author`, `title` and `xuid`
//! and `generation` 0. A tick's edits are answered with every window, the held slot selected.
// TODO: BDS's limits on a page's and a title's length are not known; only the 50 pages are kept to.

use acacia_proto::nbt::{List, Value};
use acacia_proto::packets::{BookEdit, BookEditContent};
use bevy_ecs::prelude::*;
use sapling_world::ItemStack;

use crate::components::PacketQueues;
use crate::inventory::{self, HOTBAR, Inventory, Place};

const WRITABLE: &str = "minecraft:writable_book";
const WRITTEN: &str = "minecraft:written_book";
const MAX_PAGES: usize = 50;
const COMPOUND_TAG: u8 = 10;

fn page(text: &str, photo: &str) -> Value {
    Value::Compound(vec![("photoname".into(), Value::String(photo.into())), ("text".into(), Value::String(text.into()))])
}

fn pages_of(stack: &ItemStack) -> Vec<Value> {
    match stack.tag.as_ref().and_then(|tag| tag.get("pages")) {
        Some(Value::List(list)) => list.items.clone(),
        _ => Vec::new(),
    }
}

/// `tag` with each of `set` in place of the field of its name, keys in order as BDS writes them.
fn with_fields(tag: Option<&Value>, set: Vec<(&str, Value)>) -> Value {
    let mut fields = match tag {
        Some(Value::Compound(fields)) => fields.clone(),
        _ => Vec::new(),
    };
    for (key, value) in set {
        fields.retain(|(name, _)| name != key);
        fields.push((key.into(), value));
    }
    fields.sort_by(|a, b| a.0.cmp(&b.0));
    Value::Compound(fields)
}

/// An empty list is written with element tag 0, as the client and BDS write one.
fn pages_list(pages: Vec<Value>) -> Value {
    Value::List(List { tag: if pages.is_empty() { 0 } else { COMPOUND_TAG }, items: pages })
}

/// `book` as `edit` leaves it; `None` for an edit that changes nothing it may.
fn edited(book: &ItemStack, edit: &BookEditContent) -> Option<ItemStack> {
    let mut pages = pages_of(book);
    let at = |number: i32| usize::try_from(number).ok().filter(|at| *at < MAX_PAGES);
    match edit {
        BookEditContent::ReplacePage(replace) => {
            let at = at(replace.page_number)?;
            if pages.len() <= at {
                pages.resize_with(at + 1, || page("", ""));
            }
            pages[at] = page(&replace.text, &replace.photo_name);
        }
        BookEditContent::AddPage(add) => {
            let at = at(add.page_number).filter(|at| *at <= pages.len() && pages.len() < MAX_PAGES)?;
            pages.insert(at, page(&add.text, &add.photo_name));
        }
        BookEditContent::DeletePage(delete) => drop(pages.remove(at(delete.page_number).filter(|at| *at < pages.len())?)),
        BookEditContent::SwapPages(swap) => {
            let (a, b) = (at(swap.page1).filter(|at| *at < pages.len())?, at(swap.page2).filter(|at| *at < pages.len())?);
            pages.swap(a, b);
        }
        BookEditContent::Sign(sign) => {
            let signature = vec![
                ("author", Value::String(sign.author.as_str().into())),
                ("generation", Value::Int(0)),
                ("pages", pages_list(pages)),
                ("title", Value::String(sign.title.as_str().into())),
                ("xuid", Value::String(sign.xuid.as_str().into())),
            ];
            return Some(ItemStack { name: WRITTEN.to_owned(), tag: Some(with_fields(book.tag.as_ref(), signature)), ..book.clone() });
        }
        _ => return None,
    }
    Some(ItemStack { tag: Some(with_fields(book.tag.as_ref(), vec![("pages", pages_list(pages))])), ..book.clone() })
}

/// Applies this tick's book edits to the books and quills in the hotbar slots they name.
pub(crate) fn dispatch(world: &mut World) {
    let edits = world.resource_mut::<PacketQueues>().take::<BookEdit>();
    let mut answered = Vec::new();
    for (player, edit) in edits {
        let Some(slot) = usize::try_from(edit.inventory_slot).ok().filter(|slot| *slot < HOTBAR) else { continue };
        let Some(mut inventory) = world.get_mut::<Inventory>(player) else { continue };
        let book = inventory.slot(Place::Main(slot)).and_then(Option::as_ref).map(|held| held.stack.clone()).filter(|stack| stack.name == WRITABLE);
        if let Some(new) = book.and_then(|book| edited(&book, &edit.content)) {
            inventory.put(Place::Main(slot), Some(new));
        }
        if !answered.contains(&player) {
            answered.push(player);
        }
    }
    for player in answered {
        inventory::show_windows(world, player, true);
    }
}

/// Opening a book and quill that was never written in gives it an empty `pages`, as BDS showed
/// it; false for a hand that holds none.
pub(crate) fn open(world: &mut World, player: Entity) -> bool {
    let Some(mut inventory) = world.get_mut::<Inventory>(player) else { return false };
    let Some(book) = inventory.in_hand().as_ref().map(|held| held.stack.clone()).filter(|stack| stack.name == WRITABLE) else { return false };
    if book.tag.as_ref().and_then(|tag| tag.get("pages")).is_none() {
        let slot = Place::Main(inventory.selected.into());
        inventory.put(slot, Some(ItemStack { tag: Some(with_fields(book.tag.as_ref(), vec![("pages", pages_list(Vec::new()))])), ..book }));
    }
    true
}

#[cfg(test)]
mod tests {
    use acacia_proto::packets::{BookEditContentDeletePage, BookEditContentReplacePage, BookEditContentSign};

    use super::*;

    fn book() -> ItemStack {
        ItemStack { name: WRITABLE.to_owned(), count: 1, damage: 0, block: None, tag: None }
    }

    fn texts(stack: &ItemStack) -> Vec<String> {
        pages_of(stack).iter().map(|page| if let Some(Value::String(text)) = page.get("text") { text.to_string() } else { String::new() }).collect()
    }

    fn replace(page_number: i32, text: &str) -> BookEditContent {
        BookEditContent::ReplacePage(BookEditContentReplacePage { page_number, text: text.to_owned(), photo_name: String::new() })
    }

    #[test]
    fn a_page_past_the_end_pads_and_a_signed_book_keeps_its_pages() {
        let written = edited(&book(), &replace(2, "three")).unwrap();
        assert_eq!(texts(&written), ["", "", "three"]);
        assert!(edited(&written, &BookEditContent::DeletePage(BookEditContentDeletePage { page_number: 3 })).is_none());
        assert!(edited(&book(), &replace(50, "too far")).is_none());
        let sign = BookEditContentSign { title: "Tale".to_owned(), author: "Author Unknown".to_owned(), xuid: String::new() };
        let signed = edited(&written, &BookEditContent::Sign(sign)).unwrap();
        assert_eq!((signed.name.as_str(), texts(&signed)), (WRITTEN, vec![String::new(), String::new(), "three".to_owned()]));
        let Some(Value::Compound(fields)) = &signed.tag else { panic!("a compound") };
        let keys: Vec<String> = fields.iter().map(|(key, _)| key.to_string()).collect();
        assert_eq!(keys, ["author", "generation", "pages", "title", "xuid"]);
    }
}
