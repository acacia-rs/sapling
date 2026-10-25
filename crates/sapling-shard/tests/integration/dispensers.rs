//! Dispensers, droppers and crafters fired by redstone, and their screens (docs/research/dispensers.md).

use crate::common;

use std::sync::mpsc;

use acacia_proto::RawPacket;
use acacia_proto::packets::{ContainerOpen, InventoryContent, LevelEvent, LevelEventEvent, LevelSoundEvent};
use acacia_proto::types::{ContainerSlotType as Container, ItemStackResponsesItemStatus as Status};
use common::requests::{answer, place, request, slot};
use common::{Harness, all, click};
use sapling_api::{Block, EntityInfo, Item, PlayerId, Slot};

const ALEX: PlayerId = PlayerId(1);
/// Where the firing block stands, facing east; the redstone block that fires it goes south of it.
const AT: [i32; 3] = [3, 65, 3];
const FRONT: [i32; 3] = [4, 65, 3];
const POWER: [i32; 3] = [3, 65, 4];

/// Alex on the platform, `block` at [`AT`] holding `items` from slot 0, and `more` set.
fn setup(block: Block, items: Vec<Item>, more: Vec<([i32; 3], Block)>) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| {
        assert!(ctx.update_block(AT, &block), "{block:?}");
        for (slot, item) in items.iter().enumerate() {
            assert!(ctx.set_container_item(AT, slot, Some(item)), "{item:?}");
        }
        for (pos, block) in &more {
            assert!(ctx.update_block(*pos, block), "{block:?}");
        }
    });
    ticks(&mut h, 4);
    h
}

fn ticks(h: &mut Harness, n: usize) -> Vec<RawPacket> {
    (0..n).flat_map(|_| h.tick_all().to(1).to_vec()).collect()
}

/// Powers [`AT`] (or takes the power away) and watches `n` ticks.
fn power(h: &mut Harness, on: bool, n: usize) -> Vec<RawPacket> {
    let block = Block::new(if on { "redstone_block" } else { "air" });
    h.remote.run(move |ctx| assert!(ctx.update_block(POWER, &block)));
    ticks(h, n)
}

fn entities(h: &mut Harness) -> Vec<EntityInfo> {
    let (tx, rx) = mpsc::channel();
    h.remote.run(move |ctx| tx.send(ctx.entities()).unwrap());
    h.tick_all();
    rx.recv().unwrap()
}

fn block(h: &mut Harness, pos: [i32; 3]) -> Block {
    let (tx, rx) = mpsc::channel();
    h.remote.run(move |ctx| tx.send(ctx.block(pos)).unwrap());
    h.tick_all();
    rx.recv().unwrap().expect("loaded")
}

fn events(sent: &[RawPacket]) -> Vec<LevelEventEvent> {
    all::<LevelEvent>(sent).into_iter().map(|event| event.event).collect()
}

fn sounds(sent: &[RawPacket]) -> Vec<String> {
    all::<LevelSoundEvent>(sent).into_iter().map(|sound| sound.sound_id.to_string()).collect()
}

fn lying(h: &mut Harness, name: &str) -> Vec<EntityInfo> {
    entities(h).into_iter().filter(|entity| entity.item.as_ref().is_some_and(|item| item.name == name)).collect()
}

#[test]
fn a_dropper_fires_once_on_a_rising_signal_and_throws_one_item_out_of_its_face() {
    let mut h = setup(Block::new("dropper").with("facing_direction=5,triggered_bit=0"), vec![Item::new("minecraft:diamond", 3)], vec![]);
    let fired = power(&mut h, true, 10);
    assert_eq!(sounds(&fired), ["block.click", "block.click"], "from its mouth and itself");
    assert!(events(&fired).is_empty(), "no smoke out of a dropper: {:?}", events(&fired));
    let thrown = lying(&mut h, "minecraft:diamond");
    assert_eq!(thrown.len(), 1, "one diamond out");
    assert!(thrown[0].feet[0] > 4.0 && thrown[0].velocity[0] > 0.0, "east, out of its face: {:?}", thrown[0]);
    assert_eq!(block(&mut h, AT).properties, "facing_direction=5,triggered_bit=1");
    power(&mut h, true, 10);
    assert_eq!(lying(&mut h, "minecraft:diamond").len(), 1, "a held signal fires no more");
    power(&mut h, false, 6);
    power(&mut h, true, 10);
    assert_eq!(lying(&mut h, "minecraft:diamond").len(), 2, "the next rise fires again");
}

#[test]
fn a_dropper_puts_into_the_chest_it_faces_and_an_empty_one_clicks_and_fails() {
    let mut h = setup(Block::new("dropper").with("facing_direction=5"), vec![Item::new("minecraft:emerald", 2)], vec![(FRONT, Block::new("chest"))]);
    power(&mut h, true, 10);
    assert!(lying(&mut h, "minecraft:emerald").is_empty(), "none thrown");
    h.receive(1, &click(FRONT, 1, None));
    let sent = h.tick_all().to(1).to_vec();
    let window = all::<ContainerOpen>(&sent)[0].window_id.to_raw();
    let shown = all::<InventoryContent>(&sent).into_iter().find(|content| content.window_id.to_raw() == window).expect("the chest's slots");
    assert_eq!(shown.input[0].count, 1, "one emerald in the chest");

    let mut h = setup(Block::new("dispenser").with("facing_direction=5"), vec![], vec![]);
    let failed = power(&mut h, true, 10);
    assert_eq!(sounds(&failed), ["block.click.fail"]);
}

#[test]
fn a_dispenser_shoots_an_arrow_and_a_snowball_and_primes_tnt() {
    let mut h = setup(Block::new("dispenser").with("facing_direction=5"), vec![Item::new("minecraft:arrow", 1)], vec![]);
    power(&mut h, true, 8);
    let arrows: Vec<EntityInfo> = entities(&mut h).into_iter().filter(|entity| entity.kind == "minecraft:arrow").collect();
    assert_eq!(arrows.len(), 1, "an arrow shot");
    assert!(arrows[0].feet[0] > 4.0, "east of the dispenser: {:?}", arrows[0]);

    let mut h = setup(Block::new("dispenser").with("facing_direction=5"), vec![Item::new("minecraft:snowball", 1)], vec![]);
    power(&mut h, true, 8);
    assert!(entities(&mut h).iter().any(|entity| entity.kind == "minecraft:snowball"), "a snowball thrown");

    let mut h = setup(Block::new("dispenser").with("facing_direction=5"), vec![Item::new("minecraft:tnt", 1)], vec![]);
    power(&mut h, true, 8);
    let tnt: Vec<EntityInfo> = entities(&mut h).into_iter().filter(|entity| entity.kind == "minecraft:tnt").collect();
    assert_eq!(tnt.len(), 1, "primed TNT");
    assert!((tnt[0].feet[0] - 4.5).abs() < 0.2, "in front: {:?}", tnt[0]);

    let mut h = setup(Block::new("dispenser").with("facing_direction=5"), vec![Item::new("minecraft:firework_rocket", 1)], vec![]);
    power(&mut h, true, 8);
    assert!(entities(&mut h).iter().any(|entity| entity.kind == "minecraft:fireworks_rocket" && entity.feet[0] > 4.0), "a rocket launched east");
}

#[test]
fn shears_from_a_dispenser_shear_the_sheep_in_front() {
    let mut h = setup(Block::new("dispenser").with("facing_direction=5"), vec![Item::new("minecraft:shears", 1)], vec![]);
    h.remote.run(|ctx| assert!(ctx.spawn_entity("minecraft:sheep", [4.5, 65.0, 3.5]).is_some()));
    power(&mut h, true, 8);
    assert!(!lying(&mut h, "minecraft:white_wool").is_empty(), "the sheep's wool");
}

#[test]
fn a_dispenser_pours_a_water_bucket_and_takes_the_water_back() {
    let mut h = setup(Block::new("dispenser").with("facing_direction=5"), vec![Item::new("minecraft:water_bucket", 1)], vec![]);
    power(&mut h, true, 8);
    assert_eq!(block(&mut h, FRONT).name, "minecraft:water", "the water poured");
    power(&mut h, false, 4);
    power(&mut h, true, 8);
    assert_ne!(block(&mut h, FRONT).name, "minecraft:water", "and taken back with the empty bucket");
    assert!(lying(&mut h, "minecraft:bucket").is_empty(), "the bucket was not thrown out");
}

#[test]
fn bone_meal_from_a_dispenser_grows_a_crop() {
    let mut h = setup(Block::new("dispenser").with("facing_direction=5"), vec![Item::new("minecraft:bone_meal", 4)], vec![([4, 64, 3], Block::new("farmland")), (FRONT, Block::new("wheat").with("growth=0"))]);
    power(&mut h, true, 8);
    assert_ne!(block(&mut h, FRONT).properties, "growth=0", "the wheat grew");
}

#[test]
fn a_hopper_fills_a_dispenser_and_a_player_moves_items_in_its_screen() {
    let hopper = Block::new("hopper").with("facing_direction=0,toggle_bit=0");
    let mut h = setup(Block::new("dispenser").with("facing_direction=5"), vec![], vec![([3, 66, 3], hopper)]);
    h.remote.run(|ctx| assert!(ctx.set_container_item([3, 66, 3], 0, Some(&Item::new("minecraft:arrow", 2)))));
    ticks(&mut h, 20);
    h.remote.run(|ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:snowball", 4)))));
    h.receive(1, &click(AT, 1, None));
    let sent = h.tick_all().to(1).to_vec();
    let window = all::<ContainerOpen>(&sent).first().expect("the dispenser opens").window_id.to_raw();
    let shown = all::<InventoryContent>(&sent).into_iter().find(|content| content.window_id.to_raw() == window).expect("its nine slots");
    assert_eq!(shown.input.iter().map(|item| item.count).sum::<u16>(), 2, "both arrows came down the hopper");
    h.receive(1, &request(-3, vec![place(3, slot(Container::Hotbar, 0, 0), slot(Container::Container, 8, 0))]));
    let (status, changed) = answer(h.tick_all().to(1));
    assert_eq!(status, Status::Ok, "{changed:?}");
    assert!(changed.iter().any(|(container, slot, count, _)| *container == Container::Container && *slot == 8 && *count == 3), "{changed:?}");
}

#[test]
fn a_crafter_crafts_what_its_grid_lays_out_into_the_world() {
    let planks = Item::new("minecraft:oak_planks", 1);
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| {
        assert!(ctx.update_block(AT, &Block::new("crafter").with("orientation=east_up")));
        for slot in [0, 1, 3, 4] {
            assert!(ctx.set_container_item(AT, slot, Some(&planks)));
        }
    });
    ticks(&mut h, 4);
    power(&mut h, true, 10);
    assert_eq!(lying(&mut h, "minecraft:crafting_table").len(), 1, "a crafting table out of its face");
    assert!(block(&mut h, AT).properties.contains("triggered_bit=1"));
    power(&mut h, false, 4);
    power(&mut h, true, 10);
    assert_eq!(lying(&mut h, "minecraft:crafting_table").len(), 1, "the grid was used up: nothing more");
}
