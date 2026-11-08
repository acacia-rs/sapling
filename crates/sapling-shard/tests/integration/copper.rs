//! Copper weathering by random tick, waxed with honeycomb and scraped with an axe
//! (docs/research/copper.md).

use crate::common;

use std::sync::mpsc;

use acacia_proto::nbt::Value;
use acacia_proto::packets::{ContainerOpen, LevelEvent, LevelEventEvent, UpdateSubchunkBlocks};
use acacia_proto::types::InputData;
use common::{Harness, all, click};
use sapling_api::{Block, Item, PlayerId, Slot};

const ALEX: PlayerId = PlayerId(1);

fn ready(held: &'static str, blocks: Vec<([i32; 3], Block)>) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| {
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new(held, 4))));
        for (pos, block) in &blocks {
            assert!(ctx.update_block(*pos, block), "{block:?}");
        }
    });
    h.tick_all();
    h
}

fn name(h: &mut Harness, pos: [i32; 3]) -> String {
    let (tx, rx) = mpsc::channel();
    h.remote.run(move |ctx| tx.send(ctx.block(pos).map(|block| block.name).unwrap_or_default()).unwrap());
    h.tick_all();
    rx.recv().unwrap()
}

/// Alex clicks `pos` with what it holds: the level events it is shown.
fn use_on(h: &mut Harness, pos: [i32; 3]) -> Vec<LevelEventEvent> {
    h.receive(1, &click(pos, 1, None));
    all::<LevelEvent>(h.tick_all().to(1)).into_iter().map(|event| event.event).collect()
}

#[test]
fn honeycomb_waxes_copper_and_an_axe_takes_the_wax_off_then_a_stage() {
    let mut h = ready("minecraft:honeycomb", vec![([2, 65, 2], Block::new("exposed_cut_copper")), ([3, 65, 2], Block::new("lightning_rod"))]);
    assert!(use_on(&mut h, [2, 65, 2]).contains(&LevelEventEvent::ParticleWaxOn));
    assert_eq!(name(&mut h, [2, 65, 2]), "minecraft:waxed_exposed_cut_copper");
    use_on(&mut h, [3, 65, 2]);
    assert_eq!(name(&mut h, [3, 65, 2]), "minecraft:waxed_lightning_rod", "a rod waxes too");

    let mut h = ready("minecraft:iron_axe", vec![([2, 65, 2], Block::new("waxed_exposed_cut_copper"))]);
    assert!(use_on(&mut h, [2, 65, 2]).contains(&LevelEventEvent::ParticleWaxOff));
    assert_eq!(name(&mut h, [2, 65, 2]), "minecraft:exposed_cut_copper");
    assert!(use_on(&mut h, [2, 65, 2]).contains(&LevelEventEvent::ParticleScrape));
    assert_eq!(name(&mut h, [2, 65, 2]), "minecraft:cut_copper");
}

#[test]
fn an_axe_scrapes_both_halves_of_a_copper_door() {
    let lower = "door_hinge_bit=0,minecraft:cardinal_direction=south,open_bit=0,upper_block_bit=0";
    let mut h = ready("minecraft:iron_axe", vec![([2, 65, 2], Block::new("weathered_copper_door").with(lower))]);
    assert_eq!(name(&mut h, [2, 66, 2]), "minecraft:weathered_copper_door");
    use_on(&mut h, [2, 65, 2]);
    assert_eq!([name(&mut h, [2, 65, 2]), name(&mut h, [2, 66, 2])], ["minecraft:exposed_copper_door"; 2].map(String::from));
}

const LEAD: [i32; 3] = [2, 65, 2];
const OTHER: [i32; 3] = [2, 65, 3];

fn chest_entity([x, y, z]: [i32; 3], partner: [i32; 3], leads: bool) -> sapling_api::BlockEntityData {
    let fields = [("id", Value::String("Chest".into())), ("x", Value::Int(x)), ("y", Value::Int(y)), ("z", Value::Int(z)), ("pairlead", Value::Byte(leads.into())), ("pairx", Value::Int(partner[0])), ("pairz", Value::Int(partner[2]))];
    sapling_api::BlockEntityData::new(Value::Compound(fields.into_iter().map(|(key, value)| (key.into(), value)).collect()))
}

/// A pair of `name` copper chests facing west, `LEAD` leading, Alex holding `held`.
fn chest_pair(held: &'static str, name: &'static str) -> Harness {
    let chest = Block::new(name).with("minecraft:cardinal_direction=west");
    let mut h = ready(held, vec![(LEAD, chest.clone()), (OTHER, chest)]);
    h.remote.run(|ctx| {
        assert!(ctx.set_block_entity(LEAD, &chest_entity(LEAD, OTHER, true)));
        assert!(ctx.set_block_entity(OTHER, &chest_entity(OTHER, LEAD, false)));
    });
    h.tick_all();
    h
}

fn paired(h: &mut Harness, pos: [i32; 3]) -> bool {
    common::asking::ask(h, move |ctx| ctx.block_entity(pos).and_then(|data| data.get::<Value>().map(|nbt| nbt.get("pairx").is_some()))) == Some(true)
}

#[test]
fn a_copper_chest_pair_opens_unless_sneaked_on_then_waxes_both_halves_shown_together() {
    // research/captures/bds-p3copper.txt: the clicked half's event first, both halves in one UpdateSubchunkBlocks.
    let mut h = chest_pair("minecraft:honeycomb", "copper_chest");
    h.receive(1, &click(OTHER, 1, None));
    assert!(!all::<ContainerOpen>(h.tick_all().to(1)).is_empty(), "a click without a sneak opens it");
    let mut sneak = common::input_at([0.5, 65.0, 0.5]);
    sneak.input_data = vec![InputData::StartSneaking];
    h.receive(1, &sneak);
    h.tick_all();
    h.receive(1, &click(OTHER, 1, None));
    let sent = h.tick_all().to(1).to_vec();
    let events: Vec<_> = all::<LevelEvent>(&sent).into_iter().map(|event| (event.event, event.position.z)).collect();
    assert_eq!(events, [(LevelEventEvent::ParticleWaxOn, 3.5), (LevelEventEvent::ParticleWaxOn, 2.5)]);
    assert_eq!(all::<UpdateSubchunkBlocks>(&sent).len(), 1, "both halves in one packet");
    assert_eq!([name(&mut h, LEAD), name(&mut h, OTHER)], ["minecraft:waxed_copper_chest"; 2].map(String::from));
    assert!(paired(&mut h, LEAD) && paired(&mut h, OTHER), "each half keeps its entity");
}

/// Alex puts the held `item` on the top of `on`.
fn put(h: &mut Harness, item: &'static str, on: [i32; 3]) {
    h.remote.run(move |ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new(item, 1)))));
    h.tick_all();
    h.receive(1, &click(on, 1, Some(item)));
    (0..3).for_each(|_| _ = h.tick_all());
}

#[test]
fn an_exposed_copper_chest_put_beside_a_copper_chest_takes_its_stage_and_pairs() {
    // research/captures/bds-r3copper.txt: the exposed one was shown unweathered as it was put, then paired.
    let mut h = ready("minecraft:stick", Vec::new());
    put(&mut h, "minecraft:copper_chest", [2, 64, 2]);
    let facing = common::asking::ask(&mut h, |ctx| ctx.block([2, 65, 2]).map(|block| block.properties).unwrap_or_default());
    let other = if facing.contains("north") || facing.contains("south") { [3, 65, 2] } else { [2, 65, 3] };
    put(&mut h, "minecraft:exposed_copper_chest", [other[0], 64, other[2]]);
    assert_eq!([name(&mut h, [2, 65, 2]), name(&mut h, other)], ["minecraft:copper_chest"; 2].map(String::from), "{facing}");
    assert!(paired(&mut h, [2, 65, 2]) && paired(&mut h, other));
}

#[test]
fn a_copper_chest_pair_weathers_together_by_its_lead() {
    let mut h = chest_pair("minecraft:stick", "copper_chest");
    h.remote.run(|ctx| assert!(ctx.set_game_rule("randomTickSpeed", sapling_api::RuleValue::Int(2000))));
    // Both halves read in one handler: read one a tick apart, a weathering between the reads showed a split pair.
    let both = |h: &mut Harness| common::asking::ask(h, |ctx| [LEAD, OTHER].map(|pos| ctx.block(pos).map(|block| block.name).unwrap_or_default()));
    let first = (0..600).map(|_| both(&mut h)).find(|now| now[0] != "minecraft:copper_chest" || now[1] != "minecraft:copper_chest");
    assert_eq!(first, Some(["minecraft:exposed_copper_chest"; 2].map(String::from)), "both halves at once");
    assert!(paired(&mut h, LEAD) && paired(&mut h, OTHER));
}

#[test]
fn copper_weathers_by_random_ticks_but_waxed_copper_does_not() {
    let mut h = ready("minecraft:stick", vec![([2, 65, 2], Block::new("copper_block")), ([5, 65, 5], Block::new("waxed_copper"))]);
    h.remote.run(|ctx| assert!(ctx.set_game_rule("randomTickSpeed", sapling_api::RuleValue::Int(2000))));
    let first = (0..300).map(|_| name(&mut h, [2, 65, 2])).find(|now| now != "minecraft:copper_block");
    assert_eq!(first.as_deref(), Some("minecraft:exposed_copper"), "a lone block weathers a stage at 0.068 x 0.75 a random tick");
    assert_eq!(name(&mut h, [5, 65, 5]), "minecraft:waxed_copper");
}
