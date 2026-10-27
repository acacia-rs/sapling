//! Books written and signed, and lecterns (books.rs, blocks/lectern.rs; research/captures/bds-books.txt).

use crate::common;

use acacia_proto::nbt::Value;
use acacia_proto::packets::{
    BlockEntityData, BookEdit, BookEditContent, BookEditContentReplacePage, BookEditContentSign, BookEditType, LecternUpdate, LevelSoundEvent, PlayerHotbar,
};
use acacia_proto::types::Vec3i;
use common::shooting::{held, ready};
use common::{all, click};
use sapling_api::{Block, GameMode, Slot};

fn replace(page_number: i32, text: &str) -> BookEdit {
    let content = BookEditContent::ReplacePage(BookEditContentReplacePage { page_number, text: text.to_owned(), photo_name: String::new() });
    BookEdit { inventory_slot: 0, r#type: BookEditType::ReplacePage, content }
}

fn sign(title: &str) -> BookEdit {
    let content = BookEditContent::Sign(BookEditContentSign { title: title.to_owned(), author: "Author Unknown".to_owned(), xuid: String::new() });
    BookEdit { inventory_slot: 0, r#type: BookEditType::Sign, content }
}

#[test]
fn a_book_is_written_signed_and_put_on_a_lectern_whose_page_turns() {
    let mut h = ready(GameMode::Survival, &[("minecraft:writable_book", 1, 0)], 30.0);
    h.receive(1, &replace(0, "one"));
    h.receive(1, &replace(1, "two"));
    let answered = h.tick_all().to(1).to_vec();
    assert_eq!(all::<PlayerHotbar>(&answered).iter().map(|hotbar| hotbar.select_slot).collect::<Vec<_>>(), [true], "one answer for the tick's edits");
    h.receive(1, &sign("Tale"));
    h.tick_all();
    let book = held(&mut h, Slot::Main(0)).expect("the book");
    assert_eq!(book.name, "minecraft:written_book");

    h.remote.run(|ctx| assert!(ctx.set_block([0, 65, 2], &Block::new("minecraft:lectern"))));
    h.tick_all();
    h.receive(1, &click([0, 65, 2], 1, None));
    let put = h.tick_all().to(1).to_vec();
    assert_eq!(all::<LevelSoundEvent>(&put).iter().map(|sound| sound.sound_id.as_str()).collect::<Vec<_>>(), ["item.book.put"]);
    let kept = all::<BlockEntityData>(&put).pop().expect("the lectern's entity");
    assert_eq!(kept.nbt.decode().unwrap().value.get("totalPages"), Some(&Value::Int(2)));
    assert!(held(&mut h, Slot::Main(0)).is_none(), "the book left the hand");

    h.receive(1, &LecternUpdate { page: 1, page_count: 2, position: Vec3i { x: 0, y: 65, z: 2 } });
    let turned = all::<BlockEntityData>(h.tick_all().to(1)).pop().expect("the turned page");
    assert_eq!(turned.nbt.decode().unwrap().value.get("page"), Some(&Value::Int(1)));
}
