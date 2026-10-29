//! What light does to the rules: crops, grass, ice and snow, daylight detectors and the sun, each
//! in the dark and in the light (docs/research/growth.md "Light").

use crate::common;

use std::sync::Arc;

use acacia_proto::RawPacket;
use acacia_proto::packets::{EntityEvent, EntityEventEventId};
use acacia_proto::types::Action as BlockAction;
use common::{Harness, all, block_actions};
use sapling_api::{Block, GameMode, PlayerId, RuleValue, Weather};
use sapling_world::{BlockRegistry, Chunk, ChunkPos, Dimension, MemoryStorage, StoredColumn, WorldStorage};

const ALEX: PlayerId = PlayerId(1);

fn block(name: &str, properties: &str) -> Block {
    Block { name: format!("minecraft:{name}"), properties: properties.into() }
}

/// Stone on every side of `at` and over it, and under what it stands on: the void under the
/// platform is lit from its edges.
fn walled([x, y, z]: [i32; 3]) -> Vec<[i32; 3]> {
    vec![[x - 1, y, z], [x + 1, y, z], [x, y, z - 1], [x, y, z + 1], [x, y + 1, z], [x, y - 2, z]]
}

/// Alex, the random ticks `speed` times as fast, and `set` written.
fn lit_world(speed: i32, set: Vec<([i32; 3], Block)>) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        assert!(ctx.set_game_rule("randomTickSpeed", RuleValue::Int(speed)));
        for (at, block) in set {
            assert!(ctx.set_block(at, &block), "{block:?}");
        }
    });
    h.tick_all();
    h
}

/// The blocks at `cells` after `ticks` more ticks, without the prefix.
fn blocks(h: &mut Harness, ticks: usize, cells: Vec<[i32; 3]>) -> Vec<String> {
    (0..ticks).for_each(|_| drop(h.tick_all()));
    let (sent, got) = std::sync::mpsc::channel();
    h.remote.run(move |ctx| sent.send(cells.iter().map(|at| ctx.block(*at).map(|block| format!("{}[{}]", block.name.replace("minecraft:", ""), block.properties)).unwrap_or_default()).collect()).expect("the test waits"));
    h.tick_all();
    got.recv().expect("ran")
}

fn stone(cells: Vec<[i32; 3]>) -> impl Iterator<Item = ([i32; 3], Block)> {
    cells.into_iter().map(|at| (at, block("stone", "")))
}

#[test]
fn wheat_grows_in_the_light_and_not_shut_in_the_dark() {
    let mut set = vec![([2, 64, 0], block("farmland", "")), ([2, 65, 0], block("wheat", "")), ([2, 64, 4], block("farmland", "")), ([2, 65, 4], block("wheat", ""))];
    set.extend(stone(walled([2, 65, 4])));
    let mut h = lit_world(1000, set);
    let grown = blocks(&mut h, 600, vec![[2, 65, 0], [2, 65, 4]]);
    assert!(grown[0].contains("growth=7") && grown[1].contains("growth=0"), "{grown:?}");
}

#[test]
fn wheat_stays_when_a_neighbour_changes_in_the_dark_and_in_the_light() {
    let mut set = vec![([2, 64, 0], block("farmland", "")), ([2, 65, 0], block("wheat", "")), ([2, 64, 4], block("farmland", "")), ([2, 65, 4], block("wheat", ""))];
    set.extend(stone(walled([2, 65, 0])));
    set.extend(stone(walled([2, 65, 4])).filter(|(at, _)| at[1] != 66));
    let mut h = lit_world(0, set);
    h.receive(1, &block_actions(&[(BlockAction::PredictBreak, [1, 65, 0]), (BlockAction::PredictBreak, [1, 65, 4])]));
    assert_eq!(blocks(&mut h, 2, vec![[2, 65, 0], [2, 65, 4]]), ["wheat[growth=0]", "wheat[growth=0]"]);
}

#[test]
fn a_mushroom_in_daylight_breaks_when_a_neighbour_changes_unless_on_mycelium() {
    let set = vec![([-2, 65, 0], block("brown_mushroom", "")), ([-3, 65, 0], block("stone", "")), ([-2, 64, 4], block("mycelium", "")), ([-2, 65, 4], block("brown_mushroom", "")), ([-3, 65, 4], block("stone", ""))];
    let mut h = lit_world(0, set);
    h.receive(1, &block_actions(&[(BlockAction::PredictBreak, [-3, 65, 0]), (BlockAction::PredictBreak, [-3, 65, 4])]));
    assert_eq!(blocks(&mut h, 2, vec![[-2, 65, 0], [-2, 65, 4]]), ["air[]", "brown_mushroom[]"]);
}

#[test]
fn grass_dies_under_stone_and_spreads_in_the_light_only() {
    let mut set = vec![([3, 64, 0], block("grass_block", "")), ([3, 65, 0], block("stone", "")), ([3, 64, 3], block("grass_block", "")), ([4, 64, 3], block("dirt", ""))];
    set.extend([([-3, 64, 3], block("grass_block", "")), ([-2, 64, 3], block("dirt", ""))]);
    // A room of two cells, dark, the grass and the dirt in it open to its air.
    let room = [[-4, 65, 3], [-1, 65, 3], [-3, 65, 2], [-2, 65, 2], [-3, 65, 4], [-2, 65, 4], [-3, 66, 3], [-2, 66, 3]];
    set.extend(stone(room.to_vec()));
    let mut h = lit_world(200, set);
    let after = blocks(&mut h, 1200, vec![[3, 64, 0], [4, 64, 3], [-3, 64, 3], [-2, 64, 3]]);
    assert_eq!(after.iter().map(|name| name.split('[').next().unwrap_or_default()).collect::<Vec<_>>(), ["dirt", "grass_block", "grass_block", "dirt"]);
}

#[test]
fn ice_melts_by_glowstone_and_not_in_daylight_and_snow_in_plains_melts_either_way() {
    let set = vec![
        ([3, 65, -3], block("ice", "")),
        ([4, 65, -3], block("glowstone", "")),
        ([3, 65, 5], block("snow_layer", "")),
        ([4, 65, 5], block("glowstone", "")),
        ([-5, 65, -3], block("ice", "")),
        ([-5, 65, 5], block("snow_layer", "")),
    ];
    let mut h = lit_world(1000, set);
    let after = blocks(&mut h, 300, vec![[3, 65, -3], [3, 65, 5], [-5, 65, -3], [-5, 65, 5]]);
    let names: Vec<&str> = after.iter().map(|name| name.split('[').next().unwrap_or_default()).collect();
    // Plains are over 0.15: a layer goes one random tick in 16 by day too (research/captures/bds-lightgrow.txt).
    assert!(names[0].ends_with("water") && names[1..] == ["air", "ice", "air"], "{after:?}");
}

#[test]
fn a_daylight_detector_reads_the_sky_light_in_its_cell() {
    // At noon: 15 under the open sky, the sky's 14 from the side under one block, 0 shut in.
    let mut set = vec![([0, 65, 3], block("daylight_detector", "")), ([3, 65, 3], block("daylight_detector", "")), ([3, 66, 3], block("stone", "")), ([-3, 65, 3], block("daylight_detector", ""))];
    set.extend(stone(walled([-3, 65, 3])));
    let mut h = lit_world(0, set);
    h.remote.run(|ctx| ctx.set_time(6000));
    let read = blocks(&mut h, 45, vec![[0, 65, 3], [3, 65, 3], [-3, 65, 3]]);
    let signals: Vec<&str> = read.iter().map(|state| state.split("redstone_signal=").nth(1).unwrap_or_default().trim_end_matches(']')).collect();
    assert_eq!(signals, ["15", "14", "0"], "{read:?}");
}

#[test]
fn an_inverted_detector_gives_the_dark_without_the_sun_s_angle_and_a_new_one_reads_at_once() {
    // research/captures/bds-rsplace.txt: 0, 6, 11 and 6 at these times; one put down read the next tick.
    let mut h = lit_world(0, vec![([0, 65, 3], block("daylight_detector_inverted", ""))]);
    let mut read = Vec::new();
    for time in [6000, 13000, 18000, 23000] {
        h.remote.run(move |ctx| ctx.set_time(time));
        read.extend(blocks(&mut h, 21, vec![[0, 65, 3]]));
    }
    let signals: Vec<&str> = read.iter().map(|state| state.split("redstone_signal=").nth(1).unwrap_or_default().trim_end_matches(']')).collect();
    assert_eq!(signals, ["0", "6", "11", "6"], "{read:?}");
    h.remote.run(|ctx| {
        ctx.set_time(6000);
        assert!(ctx.set_block([2, 65, 3], &block("daylight_detector", "")));
    });
    assert_eq!(blocks(&mut h, 1, vec![[2, 65, 3]]), ["daylight_detector[redstone_signal=15]"]);
}

#[test]
fn the_sun_burns_a_zombie_under_glass_and_not_under_leaves() {
    // research/captures/bds-sun.txt: glass let the sun through, leaves did not.
    let burnt = |roof: &'static str| {
        let mut h = Harness::new();
        h.enter(1, "Alex");
        h.remote.run(move |ctx| {
            ctx.set_game_mode(ALEX, GameMode::Creative);
            ctx.set_time(6000);
            for x in 3..=5 {
                for z in -1..=1 {
                    assert!(ctx.set_block([x, 68, z], &Block::new(roof)));
                }
            }
        });
        h.tick_all();
        h.remote.run(|ctx| assert!(ctx.spawn_entity("minecraft:zombie", [4.5, 65.0, 0.5]).is_some()));
        let sent: Vec<RawPacket> = (0..45).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
        all::<EntityEvent>(&sent).iter().any(|event| event.event_id == EntityEventEventId::HurtAnimation && event.data == 7)
    };
    assert_eq!([burnt("minecraft:glass"), burnt("minecraft:oak_leaves")], [true, false]);
}

/// Column (0, 0) of ice plains: still water at y 63 where x < 8, stone where not.
fn cold_floor() -> Arc<MemoryStorage> {
    let registry = BlockRegistry::vanilla();
    let mut chunk = Chunk::empty(0, 0, Dimension::overworld(registry.air_id()));
    chunk.fill_biomes(12);
    let id = |name: &str| registry.states_of(name).next().unwrap().0;
    (0..16).for_each(|x| (0..16).for_each(|z| chunk.set(x, 63, z, 0, if x < 8 { id("minecraft:water") } else { id("minecraft:stone") })));
    let storage = Arc::new(MemoryStorage::default());
    storage.store_column(ChunkPos::new(0, 0), 0, &StoredColumn::of(chunk)).unwrap();
    storage
}

#[test]
fn water_freezes_and_snow_lies_where_it_is_cold_while_it_snows() {
    let mut h = Harness::with(common::config(), sapling_world::World::with_storage(cold_floor()));
    common::stored::enter_stored(&mut h, 1);
    h.remote.run(|ctx| {
        assert!(ctx.set_game_rule("randomTickSpeed", RuleValue::Int(16)) && ctx.set_game_rule("doWeatherCycle", RuleValue::Bool(true)));
        ctx.set_weather(Weather::Rain, None);
    });
    (0..1500).for_each(|_| drop(h.tick(1)));
    let (sent, got) = std::sync::mpsc::channel();
    h.remote.run(move |ctx| {
        let count = |y: i32, xs: std::ops::Range<i32>, name: &str| xs.flat_map(|x| (0..16).map(move |z| [x, y, z])).filter(|at| ctx.block(*at).is_some_and(|block| block.name == name)).count();
        sent.send((count(63, 0..8, "minecraft:ice"), count(64, 8..16, "minecraft:snow_layer"))).expect("the test waits");
    });
    h.tick(1);
    let (ice, snow) = got.recv().expect("ran");
    assert!(ice > 20 && snow > 20, "ice {ice}, snow {snow}");
}
