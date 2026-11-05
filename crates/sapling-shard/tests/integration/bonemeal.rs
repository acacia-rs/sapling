//! Bone meal, against research/captures/bds-bonemeal.txt, and crops growing by themselves,
//! against bds-crops.txt.

use std::sync::{Arc, Mutex};

use crate::common;

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use common::{Harness, all, click};
use sapling_api::{Block, GameMode, Item, PlayerId, Slot};

const ALEX: PlayerId = PlayerId(1);

/// Alex with bone meal in hand, in creative or not, and `blocks` set.
fn fed(creative: bool, blocks: &'static [([i32; 3], &'static str)]) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| {
        ctx.set_game_mode(ALEX, if creative { GameMode::Creative } else { GameMode::Survival });
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:bone_meal", 8))));
        for (at, name) in blocks {
            assert!(ctx.set_block(*at, &Block::new(*name)));
        }
    });
    h.tick_all();
    h
}

/// Clicks the block at `at`; how many growth particles Alex was shown.
fn meal(h: &mut Harness, at: [i32; 3]) -> usize {
    h.receive(1, &click(at, 1, None));
    all::<LevelEvent>(h.tick_all().to(1)).iter().filter(|event| event.event == LevelEventEvent::ParticleCropGrowth && (event.position.x, event.position.y, event.position.z) == (at[0] as f32, at[1] as f32, at[2] as f32)).count()
}

#[test]
fn a_creative_click_ripens_a_crop_at_once_and_a_stem_in_three() {
    let mut h = fed(true, &[([2, 64, 0], "minecraft:farmland"), ([2, 65, 0], "minecraft:wheat"), ([2, 64, 2], "minecraft:farmland"), ([2, 65, 2], "minecraft:melon_stem"), ([2, 65, -2], "minecraft:sweet_berry_bush")]);
    let shown = [meal(&mut h, [2, 65, 0]), meal(&mut h, [2, 65, 0]), meal(&mut h, [2, 65, 2]), meal(&mut h, [2, 65, 2]), meal(&mut h, [2, 65, 2]), meal(&mut h, [2, 65, 2]), meal(&mut h, [2, 65, -2])];
    assert_eq!(shown, [1, 1, 1, 1, 1, 0, 1], "a ripe crop still takes it, a ripe stem does not");
    h.remote.run(|ctx| {
        let grown = [[2, 65, 0], [2, 65, 2], [2, 65, -2]].map(|at| ctx.block(at).map(|block| block.properties));
        assert!(grown[0].as_deref().is_some_and(|wheat| wheat.contains("growth=7")), "{grown:?}");
        assert!(grown[1].as_deref().is_some_and(|stem| stem.contains("growth=7")) && grown[2].as_deref().is_some_and(|bush| bush.contains("growth=3")), "{grown:?}");
        assert_eq!(ctx.item(ALEX, Slot::Main(0)).map(|held| held.count), Some(8), "creative spends none");
    });
    h.tick_all();
}

#[test]
fn grass_grows_tall_grass_sprouts_and_stone_takes_none() {
    let field: &[([i32; 3], &str)] = &[
        ([2, 64, 0], "minecraft:grass_block"), ([3, 64, 0], "minecraft:grass_block"), ([2, 64, 1], "minecraft:grass_block"), ([3, 64, 1], "minecraft:grass_block"),
        ([2, 64, -1], "minecraft:grass_block"), ([3, 64, -1], "minecraft:grass_block"), ([2, 64, 4], "minecraft:grass_block"), ([2, 65, 4], "minecraft:short_grass"),
    ];
    let mut h = fed(false, field);
    let first = meal(&mut h, [2, 64, 0]);
    // BDS's walks may leave the cell over the clicked grass free; then a second click takes again.
    let covered = Arc::new(Mutex::new(false));
    let seen = covered.clone();
    h.remote.run(move |ctx| *seen.lock().unwrap() = ctx.block([2, 65, 0]).is_some_and(|block| block.name != "minecraft:air"));
    h.tick_all();
    let covered = *covered.lock().unwrap();
    let shown = [first, meal(&mut h, [2, 64, 0]), meal(&mut h, [2, 65, 4]), meal(&mut h, [5, 64, 5])];
    assert_eq!(shown, [1, usize::from(!covered), 1, 0], "grass with a plant on it, and stone, take none");
    h.remote.run(move |ctx| {
        let named = |at: [i32; 3]| ctx.block(at).map(|block| block.name);
        assert_eq!([named([2, 65, 4]), named([2, 66, 4])].map(|name| name.unwrap_or_default()), ["minecraft:tall_grass", "minecraft:tall_grass"]);
        let taken = 2 + u8::from(!covered);
        assert_eq!(ctx.item(ALEX, Slot::Main(0)).map(|held| held.count), Some(8 - taken), "stone and covered grass refused");
    });
    h.tick_all();
}

/// The plants of one click on grass or a flower: (cell, name) of each block around `at` that is not air or stone.
fn plants_around(h: &mut Harness, at: [i32; 3]) -> Vec<([i32; 3], String)> {
    let found = Arc::new(Mutex::new(Vec::new()));
    let seen = found.clone();
    h.remote.run(move |ctx| {
        for (x, y, z) in (-4..=4).flat_map(|x| (65..=67).flat_map(move |y| (-4..=4).map(move |z| (x, y, z)))) {
            let cell = [at[0] + x, y, at[2] + z];
            if let Some(block) = ctx.block(cell).filter(|block| block.name != "minecraft:air") {
                seen.lock().unwrap().push((cell, block.name));
            }
        }
    });
    h.tick_all();
    let found = found.lock().unwrap().clone();
    found
}

/// GrassBlockBase::onFertilized (147165f00): each of BDS's walks steps over grass only, so grass a ring of
/// stone away from the clicked block gets nothing.
#[test]
fn grass_plants_only_where_its_walks_reach_over_grass() {
    const RING: &[([i32; 3], &str)] = &[
        ([0, 64, 3], "minecraft:grass_block"), ([-2, 64, 1], "minecraft:grass_block"), ([-1, 64, 1], "minecraft:grass_block"), ([0, 64, 1], "minecraft:grass_block"),
        ([1, 64, 1], "minecraft:grass_block"), ([2, 64, 1], "minecraft:grass_block"), ([-2, 64, 5], "minecraft:grass_block"), ([-1, 64, 5], "minecraft:grass_block"),
        ([0, 64, 5], "minecraft:grass_block"), ([1, 64, 5], "minecraft:grass_block"), ([2, 64, 5], "minecraft:grass_block"), ([-2, 64, 2], "minecraft:grass_block"),
        ([-2, 64, 3], "minecraft:grass_block"), ([-2, 64, 4], "minecraft:grass_block"), ([2, 64, 2], "minecraft:grass_block"), ([2, 64, 3], "minecraft:grass_block"),
        ([2, 64, 4], "minecraft:grass_block"),
    ];
    let mut h = fed(true, RING);
    for _ in 0..4 {
        meal(&mut h, [0, 64, 3]);
    }
    let plants = plants_around(&mut h, [0, 64, 3]);
    assert!(!plants.is_empty(), "the cell over the clicked grass");
    assert!(plants.iter().all(|(cell, _)| [cell[0], cell[2]] == [0, 3]), "nothing past the stone: {plants:?}");
}

/// FlowerBlock::onFertilized (14715ca10): a flower's walks start a block over it and step down onto the grass
/// around, up to three cells away; a poppy brings up poppies and now and then a dandelion.
#[test]
fn a_flower_spreads_its_kind_up_to_three_cells_over_grass() {
    let mut field: Vec<([i32; 3], &'static str)> = (-3..=3).flat_map(|x| (0..=6).map(move |z| ([x, 64, z], "minecraft:grass_block"))).collect();
    field.push(([0, 65, 3], "minecraft:poppy"));
    let mut h = fed(true, Box::leak(field.into_boxed_slice()));
    for _ in 0..6 {
        meal(&mut h, [0, 65, 3]);
    }
    let plants = plants_around(&mut h, [0, 64, 3]);
    assert!(plants.iter().all(|(cell, name)| cell[1] == 65 && (name == "minecraft:poppy" || name == "minecraft:dandelion")), "{plants:?}");
    assert!(plants.iter().all(|(cell, _)| cell[0].abs() <= 3 && (cell[2] - 3).abs() <= 3), "{plants:?}");
    assert!(plants.iter().any(|(cell, _)| cell[0].abs() >= 2 || (cell[2] - 3).abs() >= 2), "two or more away: {plants:?}");
}

#[test]
fn wheat_on_wet_farmland_ripens_by_itself() {
    // BDS's field, with the random ticks two hundred times as fast, was ripe in 20 seconds or so.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        assert!(ctx.set_game_rule("randomTickSpeed", sapling_api::RuleValue::Int(200)));
        for at in (3..=7).flat_map(|x| (-2..=2).map(move |z| [x, 64, z])) {
            assert!(ctx.set_block(at, &Block { name: "minecraft:farmland".into(), properties: "moisturized_amount=7".into() }));
            assert!(ctx.set_block([at[0], 65, at[2]], &Block::new("minecraft:wheat")));
        }
    });
    let ripe = |h: &mut Harness, ticks: usize| {
        for _ in 0..ticks {
            h.tick_all();
        }
        let (sent, got) = std::sync::mpsc::channel();
        h.remote.run(move |ctx| {
            let ripe = (3..=7).flat_map(|x| (-2..=2).map(move |z| [x, 65, z])).filter(|at| ctx.block(*at).is_some_and(|wheat| wheat.properties.contains("growth=7"))).count();
            sent.send(ripe).expect("the test waits");
        });
        h.tick_all();
        got.recv().expect("ran")
    };
    let (soon, late) = (ripe(&mut h, 100), ripe(&mut h, 1500));
    assert!(soon < 5 && late >= 20, "{soon} of 25 ripe after five seconds, {late} after eighty");
}

#[test]
fn cocoa_rooted_dirt_cane_tops_and_nether_vines_take_it_as_on_bds() {
    // research/captures/bds-t3meal.txt: cocoa a stage a click to 2, roots under rooted dirt once, cane
    // two more on its top alone, a vine grown from its tip, bamboo sixteen high none.
    let mut h = fed(false, &[([2, 65, 0], "minecraft:cocoa"), ([2, 67, 2], "minecraft:dirt_with_roots"), ([6, 65, 2], "minecraft:twisting_vines")]);
    h.remote.run(|ctx| {
        (65..=67).for_each(|y| assert!(ctx.set_block([4, y, 0], &Block::new("minecraft:reeds"))));
        (65..=80).for_each(|y| assert!(ctx.set_block([6, y, 0], &Block::new("minecraft:bamboo"))));
    });
    h.tick_all();
    let shown = [[2, 65, 0], [2, 65, 0], [2, 65, 0], [2, 67, 2], [2, 67, 2], [4, 65, 0], [4, 67, 0], [4, 67, 0], [6, 65, 0], [6, 65, 2]].map(|at| meal(&mut h, at));
    assert_eq!(shown, [1, 1, 0, 1, 0, 0, 1, 0, 0, 1]);
    let blocks = common::asking::ask(&mut h, |ctx| [[2, 65, 0], [2, 66, 2], [4, 69, 0], [4, 70, 0], [6, 66, 2]].map(|at| ctx.block(at).map(|block| format!("{}[{}]", block.name, block.properties))));
    let [cocoa, roots, cane, over, vine] = blocks.map(Option::unwrap_or_default);
    assert!(cocoa.contains("age=2") && roots == "minecraft:hanging_roots[]" && cane.starts_with("minecraft:reeds") && over == "minecraft:air[]", "{cocoa} {roots} {cane} {over}");
    assert!(vine.starts_with("minecraft:twisting_vines"), "{vine}");
}

#[test]
fn in_survival_a_crop_grows_some_stages_a_click_and_a_ripe_one_takes_none() {
    // research/captures/bds-bonemeal2.txt: wheat went 2, 4, 6, 7 and then took no more.
    let mut h = fed(false, &[([2, 64, 0], "minecraft:farmland"), ([2, 65, 0], "minecraft:wheat")]);
    let shown: Vec<usize> = (0..8).map(|_| meal(&mut h, [2, 65, 0])).collect();
    let taken = shown.iter().sum::<usize>();
    assert!((2..=4).contains(&taken) && shown[4..].iter().all(|shown| *shown == 0), "two to four stages a click to seven: {shown:?}");
    h.remote.run(move |ctx| {
        assert!(ctx.block([2, 65, 0]).is_some_and(|wheat| wheat.properties.contains("growth=7")));
        assert_eq!(ctx.item(ALEX, Slot::Main(0)).map(|held| usize::from(held.count)), Some(8 - taken), "a ripe crop spends none");
    });
    h.tick_all();
}

#[test]
fn in_survival_a_click_grows_a_crop_two_three_or_four_stages() {
    // research/captures/bds-l2meal.txt: 30 wheat at 0 went to 2, 3 and 4, never 5.
    let mut h = fed(false, &[([2, 64, 0], "minecraft:farmland")]);
    h.remote.run(|ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:bone_meal", 64)))));
    let steps: Vec<String> = (0..30)
        .map(|_| {
            h.remote.run(|ctx| assert!(ctx.set_block([2, 65, 0], &Block::new("minecraft:wheat"))));
            h.tick_all();
            meal(&mut h, [2, 65, 0]);
            common::asking::ask(&mut h, |ctx| ctx.block([2, 65, 0]).map(|wheat| wheat.properties).unwrap_or_default())
        })
        .collect();
    let seen = |step: u8| steps.iter().any(|wheat| wheat.contains(&format!("growth={step}")));
    assert!(seen(2) && seen(3) && seen(4) && !seen(5) && !seen(0), "{steps:?}");
}
