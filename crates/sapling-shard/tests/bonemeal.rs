//! Bone meal, against research/captures/bds-bonemeal.txt.

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
