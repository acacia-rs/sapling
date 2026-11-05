//! `can_destroy` and `can_place_on` stacks: kept on the stack, told on the wire, and what an
//! adventure player may break and place by them (research/captures/bds-h1lock.txt).

use crate::common;

use acacia_proto::packets::{InventoryContent, InventorySlot};
use acacia_proto::types::{Action, ItemV4Extra};
use common::asking::ask;
use common::{Harness, all, block_actions};
use sapling_api::{GameMode, Item, PlayerId, Slot};

const ALEX: PlayerId = PlayerId(1);
/// A stone of the platform next to the spawn.
const NEAR: [i32; 3] = [1, 64, 0];

fn adventurer(item: Item) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| {
        ctx.set_game_mode(ALEX, GameMode::Adventure);
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&item)));
    });
    h.tick_all();
    h
}

fn pickaxe(can_destroy: &[&str]) -> Item {
    Item { can_destroy: can_destroy.iter().map(|name| (*name).to_owned()).collect(), ..Item::new("minecraft:diamond_pickaxe", 1) }
}

/// The block at NEAR after the player took a second over breaking it.
fn after_break(h: &mut Harness) -> String {
    h.receive(1, &block_actions(&[(Action::StartBreak, NEAR)]));
    (0..20).for_each(|_| _ = h.tick_all());
    h.receive(1, &block_actions(&[(Action::PredictBreak, NEAR)]));
    h.tick_all();
    ask(h, |ctx| ctx.block(NEAR).map(|block| block.name)).expect("loaded")
}

#[test]
fn an_adventurer_breaks_only_what_its_hand_may_destroy() {
    let mut h = adventurer(pickaxe(&["minecraft:stone", "minecraft:granite"]));
    assert_eq!(after_break(&mut h), "minecraft:air");
    let mut h = adventurer(pickaxe(&["minecraft:dirt"]));
    assert_eq!(after_break(&mut h), "minecraft:stone");
}

#[test]
fn an_adventurer_places_only_on_what_its_stack_may_go_on() {
    for (on, placed) in [("minecraft:stone", "minecraft:stone"), ("minecraft:dirt", "minecraft:air")] {
        let mut h = adventurer(Item { can_place_on: vec![on.to_owned()], ..Item::new("minecraft:stone", 4) });
        h.receive(1, &common::click([3, 64, 0], 1, Some("minecraft:stone")));
        h.tick_all();
        assert_eq!(ask(&mut h, |ctx| ctx.block([3, 65, 0]).map(|block| block.name)).as_deref(), Some(placed), "can_place_on {on}");
    }
}

#[test]
fn the_lists_stay_on_the_stack_and_go_on_the_wire_apart_from_its_nbt() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let stone = Item { can_place_on: vec!["minecraft:grass_block".into()], ..Item::new("minecraft:stone", 4) };
    let given = stone.clone();
    h.remote.run(move |ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&given))));
    let sent = h.tick_all();
    let kept = ask(&mut h, |ctx| ctx.item(ALEX, Slot::Main(0)));
    assert_eq!(kept.map(|item| item.can_place_on), Some(stone.can_place_on));
    let mut slots: Vec<_> = all::<InventoryContent>(sent.to(1)).into_iter().flat_map(|content| content.input).collect();
    slots.extend(all::<InventorySlot>(sent.to(1)).into_iter().map(|slot| slot.item));
    let extra = slots.iter().find(|slot| slot.count == 4).map(|slot| &slot.extra);
    let Some(ItemV4Extra::Default(Some(extra))) = extra else { panic!("the stone's extra: {extra:?}") };
    assert_eq!((extra.can_place_on.as_slice(), extra.nbt.is_none()), (["minecraft:grass_block".to_owned()].as_slice(), true));
}
