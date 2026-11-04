//! Fire lit by hand, against research/captures/bds-fire.txt.

use crate::common;

use acacia_proto::packets::LevelSoundEvent;
use common::asking::ask;
use common::{Harness, all, click};
use sapling_api::{Block, Item, PlayerId, RuleValue, Slot, Weather};

const ALEX: PlayerId = PlayerId(1);

#[test]
fn flint_and_steel_lights_fire_over_a_block_and_soul_fire_over_soul_sand() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:flint_and_steel", 1))));
        assert!(ctx.set_block([2, 64, 2], &Block::new("minecraft:netherrack")) && ctx.set_block([2, 64, -2], &Block::new("minecraft:soul_sand")));
    });
    h.tick_all();
    for z in [0, 2, -2] {
        h.receive(1, &click([2, 64, z], 1, None));
    }
    let heard: Vec<(f32, f32, f32)> = all::<LevelSoundEvent>(h.tick_all().to(1)).into_iter().filter(|sound| sound.sound_id == "ignite").map(|sound| (sound.position.x, sound.position.y, sound.position.z)).collect();
    assert_eq!(heard, [(2.5, 65.5, 0.5), (2.5, 65.5, 2.5), (2.5, 65.5, -1.5)]);
    let lit = |h: &Harness| {
        let (sent, got) = std::sync::mpsc::channel();
        h.remote.run(move |ctx| sent.send([0, 2, -2].map(|z| ctx.block([2, 65, z]).map(|block| block.name).unwrap_or_default())).expect("the test waits"));
        got
    };
    let now = lit(&h);
    h.tick_all();
    assert_eq!(now.recv().expect("ran"), ["minecraft:fire", "minecraft:fire", "minecraft:soul_fire"]);
    // A second strike into the fire does nothing.
    h.receive(1, &click([2, 64, 0], 1, None));
    assert!(all::<LevelSoundEvent>(h.tick_all().to(1)).iter().all(|sound| sound.sound_id != "ignite"));
    for _ in 0..2000 {
        h.tick_all();
    }
    let later = lit(&h);
    h.tick_all();
    assert_eq!(later.recv().expect("ran"), ["minecraft:air", "minecraft:fire", "minecraft:soul_fire"], "fire over stone goes out");
}

#[test]
fn fire_on_planks_spreads_and_burns_them_away() {
    // BDS's three by three of planks was gone in twenty seconds.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:flint_and_steel", 1))));
        for at in (4..=6).flat_map(|x| (-1..=1).map(move |z| [x, 65, z])) {
            assert!(ctx.set_block(at, &Block::new("minecraft:oak_planks")));
        }
    });
    h.tick_all();
    h.receive(1, &click([5, 65, 0], 1, None));
    for _ in 0..1200 {
        h.tick_all();
    }
    let (sent, got) = std::sync::mpsc::channel();
    h.remote.run(move |ctx| {
        let left = (4..=6).flat_map(|x| (-1..=1).map(move |z| [x, 65, z])).filter(|at| ctx.block(*at).is_some_and(|block| block.name == "minecraft:oak_planks")).count();
        sent.send(left).expect("the test waits");
    });
    h.tick_all();
    let left = got.recv().expect("ran");
    assert!(left <= 4, "{left} of nine planks left after a minute");
}

#[test]
fn fire_goes_out_with_the_block_under_it_and_in_rain_but_not_over_netherrack() {
    // research/captures/bds-quench.txt.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let named = |h: &Harness| {
        let (sent, got) = std::sync::mpsc::channel();
        h.remote.run(move |ctx| sent.send([0, 3, 6].map(|z| ctx.block([3, 65, z]).map(|block| block.name).unwrap_or_default())).expect("the test waits"));
        got
    };
    h.remote.run(|ctx| {
        assert!(ctx.set_block([3, 64, 6], &Block::new("minecraft:netherrack")));
        for z in [0, 3, 6] {
            assert!(ctx.set_block([3, 65, z], &Block::new("minecraft:fire")));
        }
        assert!(ctx.update_block([3, 64, 0], &Block::air()));
    });
    h.tick_all();
    let unheld = named(&h);
    h.tick_all();
    assert_eq!(unheld.recv().expect("ran"), ["minecraft:air", "minecraft:fire", "minecraft:fire"]);
    h.remote.run(|ctx| ctx.set_weather(Weather::Rain, None));
    // The rain falls on blocks from its 20th tick; a fire ticks every 30 to 40.
    for _ in 0..65 {
        h.tick_all();
    }
    let rained = named(&h);
    h.tick_all();
    assert_eq!(rained.recv().expect("ran"), ["minecraft:air", "minecraft:air", "minecraft:fire"]);
}

#[test]
fn fire_and_lit_ore_go_at_the_pace_of_the_random_tick_speed_as_on_bds() {
    // research/captures/bds-firepace.txt: fire over stone and lit redstone ore, twelve of each.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let row = || (-6..6).map(|x| ([x, 65, 6], [x, 64, 3]));
    let set = |h: &mut Harness, speed: i32| ask(h, move |ctx| assert!(ctx.set_game_rule("randomTickSpeed", RuleValue::Int(speed))));
    ask(&mut h, move |ctx| {
        assert!(ctx.set_game_rule("randomTickSpeed", RuleValue::Int(0)));
        for (fire, ore) in row() {
            assert!(ctx.set_block(fire, &Block::new("minecraft:fire")) && ctx.set_block(ore, &Block::new("minecraft:lit_redstone_ore")));
        }
    });
    let left = |h: &mut Harness, ticks: usize| {
        (0..ticks).for_each(|_| drop(h.tick_all()));
        ask(h, move |ctx| {
            let is = |pos, name: &str| ctx.block(pos).is_some_and(|block| block.name == name);
            row().fold([0, 0], |[fires, ores], (fire, ore)| [fires + usize::from(is(fire, "minecraft:fire")), ores + usize::from(is(ore, "minecraft:lit_redstone_ore"))])
        })
    };
    // At 0 the queue and the random ticks stand still; at 1 a fire over stone lasts some 16 s, at 20 a second.
    assert_eq!(left(&mut h, 600), [12, 12]);
    set(&mut h, 1);
    let [fires, ores] = left(&mut h, 200);
    assert!(fires >= 8 && ores >= 8, "{fires} fires and {ores} ores of 12 left after 10 s at 1");
    set(&mut h, 20);
    assert_eq!(left(&mut h, 900), [0, 0]);
}
