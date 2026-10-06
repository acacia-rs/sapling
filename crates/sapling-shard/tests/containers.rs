//! Containers beyond the chest: hoppers moving items, shulker boxes keeping theirs, ender chests
//! holding each player's own, and furnaces paying experience and refusing what does not belong.

mod common;

use acacia_proto::RawPacket;
use acacia_proto::nbt::Value;
use acacia_proto::packets::{AddEntity, AddItemEntity, ContainerOpen, InventoryContent, LevelSoundEvent, UpdateAttributes};
use acacia_proto::types::{ContainerSlotType as Container, ItemStackResponsesItemStatus as Status, ItemV4, ItemV4Extra};
use common::requests::{answer, place, request, slot, take};
use common::{Harness, all, click};
use sapling_api::{Block, GameMode, Item, PlayerId, Slot};

const ALEX: PlayerId = PlayerId(1);
/// On the platform, beside the spawn.
const HERE: [i32; 3] = [1, 65, 0];
const THERE: [i32; 3] = [2, 65, 0];

/// Alex, and Steve when `two`, on the platform with `items` in Alex's hotbar from slot 0, and
/// `blocks` set.
fn setup(two: bool, items: Vec<Item>, blocks: Vec<([i32; 3], Block)>) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    if two {
        h.enter(2, "Steve");
    }
    h.remote.run(move |ctx| {
        for (slot, item) in items.iter().enumerate() {
            assert!(ctx.set_item(ALEX, Slot::Main(slot as u8), Some(item)));
        }
        for (pos, block) in &blocks {
            assert!(ctx.set_block(*pos, block), "{block:?}");
        }
    });
    h.tick_all();
    h
}

/// `conn` clicks the container at `pos` open: its window and what it shows.
fn open(h: &mut Harness, conn: u64, pos: [i32; 3]) -> (i64, Vec<ItemV4>) {
    h.receive(conn, &click(pos, 1, None));
    let sent = h.tick_all().to(conn).to_vec();
    let window = all::<ContainerOpen>(&sent).first().unwrap_or_else(|| panic!("{pos:?} opens")).window_id.to_raw();
    let shown = all::<InventoryContent>(&sent).into_iter().find(|content| content.window_id.to_raw() == window).expect("its slots");
    (window, shown.input)
}

/// Moves `count` from Alex's hotbar slot `from` into slot `to` of the open container named `into`.
fn put(h: &mut Harness, conn: u64, from: u8, count: u8, into: Container, to: u8) -> Status {
    h.receive(conn, &request(-3, vec![place(count, slot(Container::Hotbar, from, 0), slot(into, to, 0))]));
    answer(h.tick_all().to(conn)).0
}

fn counts(items: &[ItemV4]) -> Vec<u16> {
    items.iter().map(|item| item.count).collect()
}

fn ticks(h: &mut Harness, n: usize) -> Vec<RawPacket> {
    (0..n).flat_map(|_| h.tick(1)).collect()
}

#[test]
fn a_hopper_takes_from_the_chest_above_and_fills_the_one_it_faces() {
    let hopper = Block::new("hopper").with("facing_direction=4,toggle_bit=0");
    let mut h = setup(false, vec![Item::new("minecraft:diamond", 3)], vec![([2, 66, 0], Block::new("chest")), (THERE, hopper), (HERE, Block::new("chest"))]);
    open(&mut h, 1, [2, 66, 0]);
    assert_eq!(put(&mut h, 1, 0, 3, Container::Container, 0), Status::Ok);
    // One item a move, a move every 8 ticks: in, then out the next move.
    ticks(&mut h, 40);
    assert_eq!(counts(&open(&mut h, 1, HERE).1)[0], 3, "all three went through");
    assert_eq!(counts(&open(&mut h, 1, [2, 66, 0]).1)[0], 0);
    assert_eq!(counts(&open(&mut h, 1, THERE).1), [0; 5], "and none stayed in the hopper");

    // Powered, it holds what it is given.
    let mut h = setup(false, vec![Item::new("minecraft:diamond", 3)], vec![([2, 66, 0], Block::new("chest")), (THERE, Block::new("hopper").with("facing_direction=4,toggle_bit=1"))]);
    open(&mut h, 1, [2, 66, 0]);
    put(&mut h, 1, 0, 3, Container::Container, 0);
    ticks(&mut h, 40);
    assert_eq!(counts(&open(&mut h, 1, [2, 66, 0]).1)[0], 3);
}

/// The `Items` an item carries in its tag, as (slot, count).
fn held_items(item: &ItemV4) -> Vec<(i8, i8)> {
    let ItemV4Extra::Default(Some(extra)) = &item.extra else { return Vec::new() };
    let Some(Value::List(items)) = extra.nbt.as_ref().and_then(|nbt| nbt.nbt.value.get("Items").cloned()) else { return Vec::new() };
    items.items.iter().filter_map(|item| match (item.get("Slot"), item.get("Count")) {
        (Some(Value::Byte(slot)), Some(Value::Byte(count))) => Some((*slot, *count)),
        _ => None,
    }).collect()
}

#[test]
fn a_shulker_box_keeps_what_it_holds_when_broken_and_placed_again() {
    let items = vec![Item::new("minecraft:diamond", 5), Item::new("minecraft:red_shulker_box", 1)];
    let mut h = setup(false, items, vec![(HERE, Block::new("red_shulker_box"))]);
    open(&mut h, 1, HERE);
    assert_eq!(put(&mut h, 1, 0, 5, Container::Shulker, 4), Status::Ok);
    assert_eq!(put(&mut h, 1, 1, 1, Container::Shulker, 5), Status::Error, "no box in a box");

    h.remote.run(|ctx| ctx.set_game_mode(ALEX, GameMode::Creative));
    h.tick(1);
    h.receive(1, &common::block_actions(&[(acacia_proto::types::Action::PredictBreak, HERE)]));
    let dropped = all::<AddItemEntity>(&h.tick(1));
    assert_eq!(dropped.len(), 1, "the box, not what it held");
    assert_eq!(held_items(&dropped[0].item), [(4, 5)]);

    // Picked up into the empty first slot, then placed where it was.
    ticks(&mut h, 15);
    h.remote.run(|ctx| ctx.set_game_mode(ALEX, GameMode::Survival));
    h.tick(1);
    h.receive(1, &click([1, 64, 0], 1, None));
    ticks(&mut h, 2);
    assert_eq!(counts(&open(&mut h, 1, HERE).1)[4], 5);
}

#[test]
fn an_ender_chest_holds_each_players_own_wherever_it_is_opened() {
    let mut h = setup(true, vec![Item::new("minecraft:emerald", 7)], vec![(HERE, Block::new("ender_chest")), (THERE, Block::new("ender_chest"))]);
    open(&mut h, 1, HERE);
    assert_eq!(put(&mut h, 1, 0, 7, Container::Container, 2), Status::Ok);
    assert_eq!(counts(&open(&mut h, 2, HERE).1)[2], 0, "Steve's is empty");
    assert_eq!(counts(&open(&mut h, 1, THERE).1)[2], 7, "Alex's is in the other one too");
}

#[test]
fn a_furnace_refuses_what_does_not_belong_and_pays_for_what_is_taken() {
    let items = vec![Item::new("minecraft:raw_iron", 3), Item::new("minecraft:coal", 1), Item::new("minecraft:stone", 1)];
    let mut h = setup(false, items, vec![(HERE, Block::new("blast_furnace"))]);
    open(&mut h, 1, HERE);
    assert_eq!(put(&mut h, 1, 2, 1, Container::FurnaceFuel, 1), Status::Error, "stone does not burn");
    assert_eq!(put(&mut h, 1, 0, 3, Container::BlastFurnaceIngredient, 0), Status::Ok);
    assert_eq!(put(&mut h, 1, 1, 1, Container::FurnaceFuel, 1), Status::Ok);
    let sent = ticks(&mut h, 310);
    assert!(all::<LevelSoundEvent>(&sent).iter().any(|sound| sound.sound_id == "block.blastfurnace.fire_crackle"), "it crackles while lit");

    // Three ingots, 0.7 each: two or three points, as orbs at Alex, who takes them.
    h.receive(1, &request(-5, vec![take(3, slot(Container::FurnaceOutput, 2, 0), slot(Container::Hotbar, 0, 0))]));
    let sent = h.tick(1);
    assert_eq!(answer(&sent).0, Status::Ok);
    let orbs = all::<AddEntity>(&sent);
    assert!(!orbs.is_empty() && orbs.iter().all(|orb| orb.entity_type == "minecraft:xp_orb"), "{orbs:?}");
    let levels: Vec<UpdateAttributes> = all(&ticks(&mut h, 150));
    let progress = levels.last().and_then(|packet| packet.attributes.iter().find(|attribute| attribute.name == "minecraft:player.experience")).map(|attribute| attribute.current);
    assert!(progress.is_some_and(|progress| progress > 0.0), "{levels:?}");
}
