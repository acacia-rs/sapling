//! Copper weathering by random tick, waxed with honeycomb and scraped with an axe
//! (docs/research/copper.md).

use crate::common;

use std::sync::mpsc;

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
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

#[test]
fn copper_weathers_by_random_ticks_but_waxed_copper_does_not() {
    let mut h = ready("minecraft:stick", vec![([2, 65, 2], Block::new("copper_block")), ([5, 65, 5], Block::new("waxed_copper"))]);
    h.remote.run(|ctx| assert!(ctx.set_game_rule("randomTickSpeed", sapling_api::RuleValue::Int(2000))));
    let first = (0..300).map(|_| name(&mut h, [2, 65, 2])).find(|now| now != "minecraft:copper_block");
    assert_eq!(first.as_deref(), Some("minecraft:exposed_copper"), "a lone block weathers a stage at 0.068 x 0.75 a random tick");
    assert_eq!(name(&mut h, [5, 65, 5]), "minecraft:waxed_copper");
}
