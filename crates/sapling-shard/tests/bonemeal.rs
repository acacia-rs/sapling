//! Bone meal, against research/captures/bds-bonemeal.txt, and crops growing by themselves,
//! against bds-crops.txt.

mod common;

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
    let shown = [meal(&mut h, [2, 64, 0]), meal(&mut h, [2, 64, 0]), meal(&mut h, [2, 65, 4]), meal(&mut h, [5, 64, 5])];
    assert_eq!(shown, [1, 0, 1, 0], "grass with a plant on it, and stone, take none");
    h.remote.run(|ctx| {
        let named = |at: [i32; 3]| ctx.block(at).map(|block| block.name);
        assert_ne!(named([2, 65, 0]).as_deref(), Some("minecraft:air"));
        assert_eq!([named([2, 65, 4]), named([2, 66, 4])].map(|name| name.unwrap_or_default()), ["minecraft:tall_grass", "minecraft:tall_grass"]);
        assert_eq!(ctx.item(ALEX, Slot::Main(0)).map(|held| held.count), Some(6), "two taken, two refused");
    });
    h.tick_all();
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
fn in_survival_a_crop_grows_some_stages_a_click_and_a_ripe_one_takes_none() {
    // research/captures/bds-bonemeal2.txt: wheat went 2, 4, 6, 7 and then took no more.
    let mut h = fed(false, &[([2, 64, 0], "minecraft:farmland"), ([2, 65, 0], "minecraft:wheat")]);
    let shown: Vec<usize> = (0..8).map(|_| meal(&mut h, [2, 65, 0])).collect();
    let taken = shown.iter().sum::<usize>();
    assert!((2..=4).contains(&taken) && shown[4..].iter().all(|shown| *shown == 0), "two to five stages a click to seven: {shown:?}");
    h.remote.run(move |ctx| {
        assert!(ctx.block([2, 65, 0]).is_some_and(|wheat| wheat.properties.contains("growth=7")));
        assert_eq!(ctx.item(ALEX, Slot::Main(0)).map(|held| usize::from(held.count)), Some(8 - taken), "a ripe crop spends none");
    });
    h.tick_all();
}
