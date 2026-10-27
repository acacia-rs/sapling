//! Blocks that keep something in their entity, used by hand: bells, chiseled bookshelves and
//! cauldrons of potion (research/captures/bds-bells.txt, bds-bookshelves.txt, bds-cauldrons.txt).

use crate::common;

use std::sync::mpsc;

use acacia_proto::nbt::Value;
use acacia_proto::packets::{BlockEntityData, LevelEvent, LevelEventEvent, LevelSoundEvent};
use common::{Harness, all, click};
use sapling_api::{Block, Item, PlayerId, Slot};

const ALEX: PlayerId = PlayerId(1);
const AT: [i32; 3] = [2, 65, 2];
const UP: u8 = 1;
const WEST: u8 = 4;

fn ready(held: Item, block: Block) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| {
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&held)));
        assert!(ctx.update_block(AT, &block), "{block:?}");
    });
    h.tick_all();
    h
}

fn properties(h: &mut Harness) -> String {
    let (tx, rx) = mpsc::channel();
    h.remote.run(move |ctx| tx.send(ctx.block(AT).map(|block| block.properties).unwrap_or_default()).unwrap());
    h.tick_all();
    rx.recv().unwrap()
}

/// Alex clicks `face` of the block: what it is shown.
fn use_on(h: &mut Harness, face: u8) -> Vec<acacia_proto::RawPacket> {
    h.receive(1, &click(AT, face, None));
    h.tick_all().to(1).to_vec()
}

fn shown_field(sent: &[acacia_proto::RawPacket], key: &str) -> Option<Value> {
    all::<BlockEntityData>(sent).last().and_then(|data| data.nbt.decode().unwrap().value.get(key).cloned())
}

#[test]
fn a_bell_rings_from_a_side_but_not_from_the_top_of_a_standing_one() {
    let mut h = ready(Item::new("minecraft:stone", 1), Block::new("bell").with("attachment=standing,direction=0,toggle_bit=0"));
    let top = use_on(&mut h, UP);
    assert!(all::<LevelSoundEvent>(&top).iter().all(|sound| sound.sound_id != "block.bell.hit"), "the top rings nothing");
    let side = use_on(&mut h, WEST);
    assert!(all::<LevelSoundEvent>(&side).iter().any(|sound| sound.sound_id == "block.bell.hit"));
    assert_eq!(shown_field(&side, "Ringing"), Some(Value::Byte(1)));
    assert!(properties(&mut h).contains("toggle_bit=1"), "a click flips the bit");
}

#[test]
fn a_book_goes_into_a_shelf_slot_and_a_second_click_takes_it_back() {
    let mut h = ready(Item::new("minecraft:book", 2), Block::new("chiseled_bookshelf").with("books_stored=0,direction=1"));
    let put = use_on(&mut h, WEST);
    assert_eq!(shown_field(&put, "LastInteractedSlot"), Some(Value::Int(2)));
    assert!(properties(&mut h).contains("books_stored=2"));
    use_on(&mut h, WEST);
    assert!(properties(&mut h).contains("books_stored=0"));
}

#[test]
fn a_potion_poured_into_an_empty_cauldron_fills_two_levels_of_it() {
    let mut h = ready(Item { damage: 14, ..Item::new("minecraft:potion", 1) }, Block::new("cauldron").with("cauldron_liquid=water,fill_level=0"));
    let sent = use_on(&mut h, UP);
    assert!(all::<LevelEvent>(&sent).iter().any(|event| event.event == LevelEventEvent::CauldronFillPotion));
    assert_eq!(shown_field(&sent, "PotionId"), Some(Value::Short(14)));
    assert!(properties(&mut h).contains("fill_level=2"));
}
