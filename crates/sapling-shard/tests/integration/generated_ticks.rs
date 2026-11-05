//! Ticks a generated column's queue still holds run once it comes in (sapling-world generated/ticks.rs).

use std::sync::Arc;

use crate::common;

use common::stored::enter_stored;
use common::{Harness, config};
use sapling_api::{Block, PlayerId};
use sapling_world::{ChunkPos, Generated, MemoryStorage, StoredColumn, World, WorldStorage, decode_pending_ticks};

const SEED: i64 = 20261003;
/// Seed 20261003's chunk (5, 4) holds seven water ticks a later neighbour's flow left (sapling-worldgen liquid/leftovers.rs).
const LEFT: ChunkPos = ChunkPos { x: 5, z: 4 };
/// Seed 20261003's chunk (1, -8) holds nine bubble column cells over magma, each one cell high (sapling-world's test).
const BUBBLES: ChunkPos = ChunkPos { x: 1, z: -8 };
/// Seed 20261003's sulfur spring (sapling-world's test): its potent sulfur's chunk and cell.
const SPRING: ChunkPos = ChunkPos { x: -751, z: -752 };
const SULFUR: [i32; 3] = [-12004, 67, -12032];

/// The generated columns round `centre`.
fn generated_round(centre: ChunkPos) -> Vec<(ChunkPos, StoredColumn)> {
    let generator = Generated::with_areas(MemoryStorage::default(), SEED, 1);
    let around = (centre.x - 1..=centre.x + 1).flat_map(|x| (centre.z - 1..=centre.z + 1).map(move |z| ChunkPos::new(x, z)));
    around.map(|pos| (pos, generator.generate_column(pos))).collect()
}

/// The generated columns as storage, with or without the ticks they came with.
fn stored(generated: &[(ChunkPos, StoredColumn)], keep_ticks: bool) -> Arc<MemoryStorage> {
    let storage = Arc::new(MemoryStorage::default());
    for (pos, column) in generated {
        let pending_ticks = if keep_ticks { column.pending_ticks.clone() } else { Vec::new() };
        storage.store_column(*pos, 0, &StoredColumn { pending_ticks, ..column.clone() }).unwrap();
    }
    storage
}

/// A player that has stood over `centre` for `ticks` ticks.
fn standing(storage: Arc<MemoryStorage>, centre: ChunkPos, ticks: usize) -> Harness {
    let mut h = Harness::with(config(), World::with_storage(storage));
    enter_stored(&mut h, 1);
    h.remote.run(move |ctx| ctx.teleport(PlayerId(1), [centre.x as f32 * 16.0 + 8.5, 120.0, centre.z as f32 * 16.0 + 8.5]));
    (0..ticks).for_each(|_| _ = h.tick(1));
    h
}

/// The blocks at `cells` once a player has stood over `centre` for `ticks` ticks.
fn states_after(storage: Arc<MemoryStorage>, centre: ChunkPos, ticks: usize, cells: Vec<[i32; 3]>) -> Vec<Option<Block>> {
    let mut h = standing(storage, centre, ticks);
    let (sent, got) = std::sync::mpsc::channel();
    h.remote.run(move |ctx| sent.send(cells.iter().map(|&at| ctx.block(at)).collect::<Vec<_>>()).expect("the test waits"));
    h.tick(1);
    got.recv().expect("ran")
}

/// The block names at `cells` ([`states_after`]).
fn blocks_after(storage: Arc<MemoryStorage>, centre: ChunkPos, ticks: usize, cells: Vec<[i32; 3]>) -> Vec<Option<String>> {
    states_after(storage, centre, ticks, cells).into_iter().map(|block| block.map(|block| block.name)).collect()
}

#[test]
fn liquid_a_generated_column_was_left_waiting_on_flows_once_it_comes_in() {
    let generated = generated_round(LEFT);
    let waiting: Vec<[i32; 3]> = generated.iter().flat_map(|(_, column)| decode_pending_ticks(&column.pending_ticks).unwrap().1).filter(|tick| tick.name.ends_with("water")).map(|tick| tick.pos).collect();
    assert!(!waiting.is_empty(), "the columns round {LEFT:?} were left with liquid ticks");
    let cells: Vec<[i32; 3]> = waiting.iter().flat_map(|&[x, y, z]| [[x - 1, y, z], [x + 1, y, z], [x, y - 1, z], [x, y, z - 1], [x, y, z + 1]]).collect();
    let (ticked, still) = (blocks_after(stored(&generated, true), LEFT, 40, cells.clone()), blocks_after(stored(&generated, false), LEFT, 40, cells.clone()));
    let liquid = |name: &Option<String>| name.as_deref().is_some_and(|name| name.ends_with("water") || name.ends_with("lava"));
    let flowed = cells.iter().zip(ticked.iter().zip(&still)).filter(|(_, (ticked, still))| ticked != still && liquid(ticked)).count();
    assert!(flowed > 0, "liquid spread beside the waiting cells {waiting:?} only with their ticks: {ticked:?} against {still:?}");
}

/// BDS's tick-free world keeps a tick on each one-cell column decoration made, so the column climbs in game.
#[test]
fn a_generated_bubble_column_climbs_once_its_column_comes_in() {
    let generated = generated_round(BUBBLES);
    let over = [[17, 7, -122], [17, 8, -122]];
    let ticked = blocks_after(stored(&generated, true), BUBBLES, 20, over.to_vec());
    let still = blocks_after(stored(&generated, false), BUBBLES, 20, over.to_vec());
    let column = Some("minecraft:bubble_column".to_string());
    assert_eq!(ticked, [column.clone(), column], "over the column at 17,6,-122");
    assert!(still.iter().all(|name| name.as_deref() == Some("minecraft:water")), "{still:?}");
}

/// The sulfur spring's potent sulfur waits 85, 309 and 369 ticks; under ice it dries at the first, as BDS's
/// did on the canon world (research/captures/bds-af2spring.txt).
#[test]
fn a_generated_spring_s_potent_sulfur_dries_under_its_ice_when_its_tick_runs() {
    let generated = generated_round(SPRING);
    let state = |ticks: bool| states_after(stored(&generated, ticks), SPRING, 160, vec![SULFUR, [SULFUR[0], SULFUR[1] + 1, SULFUR[2]]]);
    let (ticked, still) = (state(true), state(false));
    let block = |name: &str, properties: &str| Some(Block { name: name.into(), properties: properties.into() });
    assert_eq!(ticked, [block("minecraft:potent_sulfur", "potent_sulfur_state=dry"), block("minecraft:ice", "")]);
    assert_eq!(still[0], block("minecraft:potent_sulfur", "potent_sulfur_state=dormant"));
}

/// What waits on the potent sulfur is saved with its column, its time gone off each delay.
#[test]
fn a_generated_spring_s_potent_sulfur_ticks_are_kept_on_save() {
    let generated = generated_round(SPRING);
    let storage = stored(&generated, true);
    let mut h = standing(storage.clone(), SPRING, 60);
    h.shard.save_worlds();
    let kept = storage.load_column(SPRING, 0).unwrap().unwrap().pending_ticks;
    let (saved_at, ticks) = decode_pending_ticks(&kept).unwrap();
    let waits: Vec<(&str, [i32; 3], i64)> = ticks.iter().map(|tick| (tick.name.as_str(), tick.pos, tick.time - saved_at)).collect();
    assert_eq!(waits.len(), 3, "{waits:?}");
    // Each is as far off as the level's clock has run since the column's date (0).
    let gone: Vec<i64> = waits.iter().zip([85, 309, 369]).map(|(&(_, _, wait), delay)| delay - wait).collect();
    assert!(waits.iter().all(|&(name, pos, _)| name == "minecraft:potent_sulfur" && pos == SULFUR), "{waits:?}");
    assert!(gone.iter().all(|&gone| gone == saved_at && (60..85).contains(&gone)), "{waits:?} saved at {saved_at}");
}
