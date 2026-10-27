//! Kelp, cave vines, mushrooms and stalactites' drips (blocks/growth/kelp.rs, wild.rs, drips.rs;
//! research/captures/bds-plants.txt, bds-plantmeal.txt, bds-dripstone.txt).

use crate::common;

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use common::{Harness, all};
use sapling_api::{Block, ShardCtx};

fn set(ctx: &mut ShardCtx<'_>, pos: [i32; 3], block: Block) {
    assert!(ctx.set_block(pos, &block), "{block:?} at {pos:?}");
}

fn names(h: &mut Harness, cells: Vec<[i32; 3]>) -> Vec<String> {
    let (sent, got) = std::sync::mpsc::channel();
    h.remote.run(move |ctx| sent.send(cells.iter().map(|at| ctx.block(*at).map_or(String::new(), |block| format!("{}[{}]", block.name, block.properties))).collect::<Vec<_>>()).unwrap());
    h.tick(1);
    got.recv().unwrap()
}

fn speed(h: &mut Harness, speed: i32) {
    h.remote.run(move |ctx| assert!(ctx.set_game_rule("randomTickSpeed", sapling_api::RuleValue::Int(speed))));
    h.tick(1);
}

#[test]
fn kelp_climbs_its_water_column_on_the_random_queue_and_stops_at_the_top() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        (65..=70).for_each(|y| set(ctx, [4, y, 0], Block::new("water")));
        set(ctx, [4, 65, 0], Block::new("kelp").with("kelp_age=1"));
    });
    h.tick(1);
    speed(&mut h, 1000);
    (0..400).for_each(|_| _ = h.tick(1));
    let grown = names(&mut h, (66..=71).map(|y| [4, y, 0]).collect());
    assert!(grown[..5].iter().all(|name| name.starts_with("minecraft:kelp")), "{grown:?}");
    assert!(!grown[5].starts_with("minecraft:kelp"), "no water over the top: {grown:?}");
}

#[test]
fn cave_vines_grow_down_from_their_tip() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        set(ctx, [4, 80, 0], Block::new("stone"));
        set(ctx, [4, 79, 0], Block::new("cave_vines").with("growing_plant_age=1"));
    });
    h.tick(1);
    speed(&mut h, 2000);
    (0..300).for_each(|_| _ = h.tick(1));
    let vines = names(&mut h, vec![[4, 78, 0], [4, 77, 0]]);
    assert!(vines.iter().all(|name| name.starts_with("minecraft:cave_vines")), "{vines:?}");
}

#[test]
fn a_stalactite_under_water_drips_into_the_cauldron_under_it() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        set(ctx, [4, 72, 0], Block::new("water"));
        set(ctx, [4, 71, 0], Block::new("stone"));
        set(ctx, [4, 70, 0], Block::new("pointed_dripstone").with("dripstone_thickness=frustum,hanging=1"));
        set(ctx, [4, 69, 0], Block::new("pointed_dripstone").with("dripstone_thickness=tip,hanging=1"));
        set(ctx, [4, 65, 0], Block::new("cauldron"));
    });
    h.tick(1);
    speed(&mut h, 1000);
    let sent: Vec<_> = (0..200).flat_map(|_| h.tick(1)).collect();
    let events = all::<LevelEvent>(&sent);
    assert!(events.iter().any(|event| event.event == LevelEventEvent::ParticleDripstoneDrip), "it dripped");
    assert!(events.iter().any(|event| event.event == LevelEventEvent::CauldronFillWater), "the cauldron filled");
    let cauldron = names(&mut h, vec![[4, 65, 0]]);
    assert!(cauldron[0].contains("cauldron_liquid=water") && !cauldron[0].contains("fill_level=0"), "{cauldron:?}");
}
