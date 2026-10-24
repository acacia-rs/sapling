//! Items that name no block: what they place, what that block holds, and what it leaves.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::nbt::{Nbt, Value};
use acacia_proto::packets::{AddItemEntity, BlockEntityData, LevelEvent, LevelEventEvent, LevelSoundEvent, OpenSign, UpdateBlock};
use acacia_proto::types::{Action, BlockCoordinates};
use common::{Harness, all, block_actions, click, input_at};
use sapling_api::{Item, PlayerId, Slot};

const ALEX: PlayerId = PlayerId(1);
const NEAR: [i32; 3] = [1, 64, 0];
const ABOVE: [i32; 3] = [1, 65, 0];

/// Alex, in survival, with `item` in the hand, and Steve watching.
fn with_in_hand(item: Item) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.remote.run(move |ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&item))));
    h.tick_all();
    h
}

/// Breaks the block at `pos` by hand in survival, giving it `ticks`; what Steve sees left there.
fn break_by_hand(h: &mut Harness, pos: [i32; 3], ticks: usize) -> Vec<AddItemEntity> {
    h.receive(1, &block_actions(&[(Action::StartBreak, pos)]));
    (0..ticks).for_each(|_| drop(h.tick_all()));
    h.receive(1, &block_actions(&[(Action::PredictBreak, pos)]));
    all::<AddItemEntity>(h.tick_all().to(2))
}

/// The block entities among `packets` as (position, entity).
fn entities(packets: &[RawPacket]) -> Vec<([i32; 3], Value)> {
    all::<BlockEntityData>(packets).into_iter().map(|data| ([data.position.x, data.position.y, data.position.z], data.nbt.decode().unwrap().value)).collect()
}

fn text(entity: &Value, side: &str) -> Option<Value> {
    entity.get(side)?.get("Text").cloned()
}

#[test]
fn a_placed_sign_opens_its_editor_and_keeps_what_its_placer_writes() {
    let mut h = with_in_hand(Item::new("minecraft:oak_sign", 2));
    h.receive(1, &click(NEAR, 1, None));
    let sent = h.tick_all();
    let opened = all::<OpenSign>(sent.to(1));
    assert_eq!(opened.iter().map(|open| ([open.position.x, open.position.y, open.position.z], open.is_front)).collect::<Vec<_>>(), [(ABOVE, true)]);
    let shown = entities(sent.to(2));
    assert_eq!((shown[0].0, shown[0].1.get("id")), (ABOVE, Some(&Value::String("Sign".into()))));
    assert_eq!(text(&shown[0].1, "FrontText"), Some(Value::String(Default::default())));

    // Steve was shown no editor: what his client says about the sign is ignored.
    let written = |words: &str| {
        let side = Value::Compound(vec![("Text".into(), Value::String(words.into()))]);
        let nbt = Nbt { name: String::new(), value: Value::Compound(vec![("FrontText".into(), side)]) };
        BlockEntityData { position: BlockCoordinates { x: ABOVE[0], y: ABOVE[1], z: ABOVE[2] }, nbt: nbt.into() }
    };
    h.receive(2, &written("Steve was here"));
    assert!(entities(h.tick_all().to(1)).is_empty());
    h.receive(1, &written("Hello"));
    let shown = entities(h.tick_all().to(2));
    assert_eq!(text(&shown[0].1, "FrontText"), Some(Value::String("Hello".into())));
    assert_eq!(text(&shown[0].1, "BackText"), Some(Value::String(Default::default())), "the back is as it was");
    h.receive(1, &written("Again"));
    assert!(entities(h.tick_all().to(2)).is_empty(), "the editor was closed");

    // A click opens it again, at the side the clicker stands on: the sign faces north.
    for (z, front) in [(-1.5, true), (2.5, false)] {
        h.receive(1, &input_at([1.5, 65.0, z]));
        h.tick_all();
        h.receive(1, &click(ABOVE, 2, None));
        let opened = all::<OpenSign>(h.tick_all().to(1));
        assert_eq!(opened.iter().map(|open| open.is_front).collect::<Vec<_>>(), [front]);
    }
    h.receive(1, &input_at([0.5, 65.0, 0.5]));
    h.tick_all();

    // On a wall it is a wall sign.
    h.remote.run(|ctx| assert!(ctx.set_block([2, 65, 1], &sapling_api::Block::new("stone"))));
    h.tick_all();
    h.receive(1, &click([2, 65, 1], 4, None));
    let placed: Vec<u32> = all::<UpdateBlock>(h.tick_all().to(2)).iter().map(|update| update.block_runtime_id).collect();
    let registry = sapling_world::BlockRegistry::vanilla();
    let names: Vec<_> = placed.iter().filter_map(|hash| registry.get(registry.runtime_id_from_hash(*hash)?)).map(|state| (state.name, state.properties)).collect();
    assert_eq!(names, [("minecraft:wall_sign", "facing_direction=4")]);
}

#[test]
fn a_dye_colours_a_written_sign_and_a_waxed_one_only_sounds() {
    let mut h = with_in_hand(Item::new("minecraft:oak_sign", 1));
    h.receive(1, &click(NEAR, 1, None));
    h.tick_all();
    let side = Value::Compound(vec![("Text".into(), Value::String("Hello".into()))]);
    let nbt = Nbt { name: String::new(), value: Value::Compound(vec![("FrontText".into(), side)]) };
    h.receive(1, &BlockEntityData { position: BlockCoordinates { x: ABOVE[0], y: ABOVE[1], z: ABOVE[2] }, nbt: nbt.into() });
    h.tick_all();
    // In front of the sign, which faces north.
    h.receive(1, &input_at([1.5, 65.0, -1.5]));
    h.tick_all();

    let hold = |h: &mut Harness, item: &'static str| {
        h.remote.run(move |ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new(item, 1)))));
        h.tick_all();
        h.receive(1, &click(ABOVE, 2, None));
        h.tick_all()
    };
    let sent = hold(&mut h, "minecraft:red_dye");
    let shown = entities(sent.to(2));
    assert_eq!(shown[0].1.get("FrontText").and_then(|side| side.get("SignTextColor")), Some(&Value::Int(-5_231_066)));
    assert_eq!(all::<LevelEvent>(sent.to(2)).iter().map(|event| event.event).collect::<Vec<_>>(), [LevelEventEvent::DyeUsed]);
    assert!(all::<OpenSign>(sent.to(1)).is_empty(), "no editor");

    hold(&mut h, "minecraft:honeycomb");
    let sent = hold(&mut h, "minecraft:blue_dye");
    assert!(entities(sent.to(2)).is_empty() && all::<OpenSign>(sent.to(1)).is_empty(), "waxed: nothing changes");
    assert_eq!(all::<LevelSoundEvent>(sent.to(2)).iter().map(|sound| sound.sound_id.as_str()).collect::<Vec<_>>(), ["block.sign.waxed_interact_fail"]);
}

#[test]
fn pots_cauldrons_campfires_brewing_stands_and_skulls_get_their_entities() {
    let kinds = [
        ("minecraft:flower_pot", "FlowerPot"),
        ("minecraft:cauldron", "Cauldron"),
        ("minecraft:campfire", "Campfire"),
        ("minecraft:brewing_stand", "BrewingStand"),
        ("minecraft:skeleton_skull", "Skull"),
    ];
    for (item, id) in kinds {
        let mut h = with_in_hand(Item::new(item, 1));
        h.receive(1, &click(NEAR, 1, Some(item).filter(|item| item.ends_with("skull"))));
        let shown = entities(h.tick_all().to(2));
        assert_eq!(shown.iter().map(|(pos, entity)| (*pos, entity.get("id").cloned())).collect::<Vec<_>>(), [(ABOVE, Some(Value::String(id.into())))], "{item}");
    }
}

#[test]
fn a_banner_keeps_its_colour_from_item_to_block_and_back() {
    const RED: i16 = 1;
    let mut h = with_in_hand(Item { damage: RED, ..Item::new("minecraft:banner", 1) });
    h.receive(1, &click(NEAR, 1, None));
    let shown = entities(h.tick_all().to(2));
    assert_eq!(shown[0].1.get("Base"), Some(&Value::Int(RED.into())));
    let left = break_by_hand(&mut h, ABOVE, 40);
    assert_eq!(left.iter().map(|item| (item.item.count, item.item.metadata)).collect::<Vec<_>>(), [(1, RED as u32)]);
}

#[test]
fn a_broken_bed_leaves_a_bed_of_its_colour() {
    const RED: i16 = 14;
    let mut h = with_in_hand(Item { damage: RED, ..Item::new("minecraft:bed", 1) });
    h.receive(1, &click(NEAR, 1, None));
    h.tick_all();
    let left = break_by_hand(&mut h, ABOVE, 10);
    assert_eq!(left.iter().map(|item| (item.item.count, item.item.metadata)).collect::<Vec<_>>(), [(1, RED as u32)]);
}
