//! Breaking, placing and clicking blocks: what reaches the world and the viewers, and what a
//! refused request gets back (harness: common/mod.rs).

mod common;

use std::sync::{Arc, Mutex};

use acacia_proto::RawPacket;
use acacia_proto::packets::{LevelEvent, LevelEventEvent, LevelSoundEvent, UpdateBlock};
use acacia_proto::types::Action;
use common::{Harness, all, block_actions, click, config, hash, platform};
use sapling_api::{Block, BreakEvent, GameMode, Handler, InteractEvent, PlaceEvent, Player, PlayerId, ShardCtx};
use sapling_shard::ShardConfig;
use sapling_world::{MemoryStorage, World};

const STONE: &str = "minecraft:stone";
const DIRT: &str = "minecraft:dirt";
const AIR: &str = "minecraft:air";
const WATER: &str = "minecraft:water";
/// A block of the platform next to the spawn, and the air above it.
const NEAR: [i32; 3] = [1, 64, 0];
const ABOVE: [i32; 3] = [1, 65, 0];
const UP: u8 = 1;

/// Every UpdateBlock among `packets` as (position, block, layer).
fn updates(packets: &[RawPacket]) -> Vec<([i32; 3], u32, u32)> {
    all::<UpdateBlock>(packets).iter().map(|u| ([u.position.x, u.position.y, u.position.z], u.block_runtime_id, u.layer)).collect()
}

/// Whether `packets` say nothing about blocks (a mover's viewers still hear of its input).
fn quiet(packets: &[RawPacket]) -> bool {
    updates(packets).is_empty() && all::<LevelEvent>(packets).is_empty() && all::<LevelSoundEvent>(packets).is_empty()
}

/// Both layers of a position, as a resync sends them.
fn shown(pos: [i32; 3], block: &str) -> [([i32; 3], u32, u32); 2] {
    [(pos, hash(block), 0), (pos, hash(AIR), 1)]
}

fn block(h: &mut Harness, pos: [i32; 3]) -> Option<Block> {
    let seen = Arc::new(Mutex::new(None));
    let out = seen.clone();
    h.remote.run(move |ctx| *out.lock().unwrap() = ctx.block(pos));
    h.tick_all();
    seen.lock().unwrap().take()
}

fn name(h: &mut Harness, pos: [i32; 3]) -> String {
    block(h, pos).expect("a loaded position").name
}

/// Two players at the spawn of the platform, the first in `mode`.
fn two_players(mode: GameMode) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.remote.run(move |ctx| ctx.set_game_mode(PlayerId(1), mode));
    h.tick_all();
    h
}

/// Cancels every break and place below x 0 and logs the clicks it is shown.
#[derive(Clone, Default)]
struct WestIsProtected(Arc<Mutex<Vec<String>>>);

impl Handler for WestIsProtected {
    fn on_break(&mut self, _: &mut ShardCtx<'_>, _: &Player, event: &mut BreakEvent) {
        if event.pos()[0] < 0 {
            event.cancel();
        }
    }

    fn on_place(&mut self, _: &mut ShardCtx<'_>, _: &Player, event: &mut PlaceEvent) {
        if event.pos()[0] < 0 {
            event.cancel();
        }
    }

    fn on_interact(&mut self, _: &mut ShardCtx<'_>, player: &Player, event: &mut InteractEvent) {
        self.0.lock().unwrap().push(format!("{} clicked {} at {:?} {:?}", player.name, event.block().name, event.pos(), event.face()));
    }
}

#[test]
fn a_break_changes_the_world_for_everyone_who_sees_the_block() {
    let mut h = two_players(GameMode::Survival);
    h.receive(1, &block_actions(&[(Action::StartBreak, NEAR)]));
    let sent = h.tick_all();
    for conn in [1, 2] {
        let hits = all::<LevelEvent>(sent.to(conn));
        assert_eq!((hits.len(), hits[0].event, hits[0].data), (1, LevelEventEvent::ParticlePunchBlockUp, hash(STONE) as i32));
        assert!(updates(sent.to(conn)).is_empty(), "a started break changes nothing");
    }

    // The same break named twice, as a client does, is one break.
    h.receive(1, &block_actions(&[(Action::ContinueBreak, NEAR), (Action::PredictBreak, NEAR), (Action::PredictBreak, NEAR)]));
    let sent = h.tick_all();
    for conn in [1, 2] {
        let burst = all::<LevelEvent>(sent.to(conn));
        let at = &burst[0].position;
        assert_eq!((burst.len(), burst[0].event, [at.x, at.y, at.z], burst[0].data), (1, LevelEventEvent::ParticleDestroy, [1.5, 64.5, 0.5], hash(STONE) as i32));
        let update = all::<UpdateBlock>(sent.to(conn));
        assert_eq!((update.len(), update[0].flags.0), (1, 3), "one update with BDS's flags");
        assert_eq!(updates(sent.to(conn)), [(NEAR, hash(AIR), 0)]);
    }
    assert_eq!(name(&mut h, NEAR), AIR);
}

#[test]
fn a_waterlogged_block_leaves_its_water_behind() {
    let mut level = platform();
    let water = level.block_id(WATER).unwrap();
    level.set_block(1, 65, 0, level.block_id("minecraft:iron_bars").unwrap());
    assert!(level.set_liquid(1, 65, 0, water));
    let mut h = Harness::with(config(), level);
    h.enter(1, "Alex");
    h.receive(1, &block_actions(&[(Action::PredictBreak, ABOVE)]));
    assert_eq!(updates(&h.tick(1)), [(ABOVE, hash(WATER), 0), (ABOVE, hash(AIR), 1)]);
    assert_eq!(name(&mut h, ABOVE), WATER);
    // Water is not something to break.
    h.receive(1, &block_actions(&[(Action::PredictBreak, ABOVE)]));
    assert_eq!(updates(&h.tick(1))[..2], [(ABOVE, hash(WATER), 0), (ABOVE, hash(AIR), 1)]);
}

#[test]
fn a_cancelled_break_is_undone_for_the_breaker_and_tells_nobody_else() {
    let mut h = two_players(GameMode::Survival);
    h.shard.add_handler(Box::new(WestIsProtected::default()));
    h.receive(1, &block_actions(&[(Action::PredictBreak, [-1, 64, 0])]));
    let sent = h.tick_all();
    let both = [shown([-1, 64, 0], STONE), shown([-1, 65, 0], AIR)].concat();
    assert_eq!(updates(sent.to(1)), both, "the block and what stood on it, on both layers");
    assert!(quiet(sent.to(2)) && all::<LevelEvent>(sent.to(1)).is_empty());
    assert_eq!(name(&mut h, [-1, 64, 0]), STONE);
}

#[test]
fn adventure_and_spectator_break_and_place_nothing() {
    for mode in [GameMode::Adventure, GameMode::Spectator] {
        let mut h = two_players(mode);
        h.receive(1, &block_actions(&[(Action::StartBreak, NEAR), (Action::PredictBreak, NEAR)]));
        h.receive(1, &click(NEAR, UP, Some(DIRT)));
        let sent = h.tick_all();
        let expected = [shown(NEAR, STONE), shown(ABOVE, AIR), shown(NEAR, STONE), shown(ABOVE, AIR)].concat();
        assert_eq!(updates(sent.to(1)), expected, "{mode:?}: the break, then the place");
        assert!(updates(sent.to(2)).is_empty() && all::<LevelSoundEvent>(sent.to(2)).is_empty(), "{mode:?}: at most the hit is seen");
        assert_eq!((name(&mut h, NEAR), name(&mut h, ABOVE)), (STONE.to_owned(), AIR.to_owned()));
    }
}

#[test]
fn a_place_in_creative_puts_the_held_block_on_the_clicked_face() {
    let mut h = two_players(GameMode::Creative);
    h.shard.add_handler(Box::new(WestIsProtected::default()));
    h.receive(1, &click(NEAR, UP, Some(DIRT)));
    let sent = h.tick_all();
    for conn in [1, 2] {
        assert_eq!(updates(sent.to(conn)), [(ABOVE, hash(DIRT), 0)]);
        let sounds = all::<LevelSoundEvent>(sent.to(conn));
        let at = &sounds[0].position;
        assert_eq!((sounds.len(), sounds[0].sound_id.as_str(), [at.x, at.y, at.z], sounds[0].extra_data), (1, "place", [1.5, 65.5, 0.5], hash(DIRT) as i32));
    }
    assert_eq!(name(&mut h, ABOVE), DIRT);

    // A side face, and a cell that is taken: the first builds east of the dirt, the second is refused.
    h.receive(1, &click(ABOVE, 5, Some(STONE)));
    assert_eq!(updates(h.tick_all().to(2)), [([2, 65, 0], hash(STONE), 0)]);
    h.receive(1, &click(NEAR, UP, Some(STONE)));
    let sent = h.tick_all();
    assert_eq!(updates(sent.to(1)), [shown(NEAR, STONE), shown(ABOVE, DIRT)].concat());
    assert!(quiet(sent.to(2)));
}

#[test]
fn a_refused_place_shows_the_clicked_block_and_its_neighbour_on_both_layers() {
    // Cancelled by a handler in creative, and refused in survival, where nobody vouches for the item.
    for (mode, against) in [(GameMode::Creative, [-1, 64, 0]), (GameMode::Survival, NEAR)] {
        let mut h = two_players(mode);
        h.shard.add_handler(Box::new(WestIsProtected::default()));
        let target = [against[0], 65, 0];
        h.receive(1, &click(against, UP, Some(DIRT)));
        let sent = h.tick_all();
        assert_eq!(updates(sent.to(1)), [shown(against, STONE), shown(target, AIR)].concat(), "{mode:?}");
        assert!(quiet(sent.to(2)), "{mode:?}: nobody else hears of it");
        assert_eq!(name(&mut h, target), AIR);
    }
}

#[test]
fn nobody_builds_into_a_player() {
    let mut h = two_players(GameMode::Creative);
    // Both stand at the spawn, in the cell above the block under their feet.
    h.receive(1, &click([0, 64, 0], UP, Some(DIRT)));
    let sent = h.tick_all();
    assert_eq!(updates(sent.to(1)), [shown([0, 64, 0], STONE), shown([0, 65, 0], AIR)].concat());
    assert!(quiet(sent.to(2)));
}

#[test]
fn a_click_that_places_nothing_is_an_interaction_and_shows_the_blocks_again() {
    let mut h = two_players(GameMode::Creative);
    let seen = WestIsProtected::default();
    h.shard.add_handler(Box::new(seen.clone()));
    let lever = [2, 65, 0];
    h.remote.run(move |ctx| assert!(ctx.set_block(lever, &Block::new("lever"))));
    assert_eq!(updates(h.tick_all().to(2)), [(lever, hash("minecraft:lever"), 0)], "a handler's block reaches the viewers");

    // An empty hand on stone, then a block in hand on a lever: neither builds.
    h.receive(1, &click(NEAR, UP, None));
    h.receive(1, &click(lever, UP, Some(DIRT)));
    let sent = h.tick_all();
    let expected = [shown(NEAR, STONE), shown(ABOVE, AIR), shown([1, 63, 0], AIR), shown(lever, "minecraft:lever"), shown([2, 66, 0], AIR), shown([2, 64, 0], STONE)].concat();
    assert_eq!(updates(sent.to(1)), expected);
    assert!(quiet(sent.to(2)));
    assert_eq!(*seen.0.lock().unwrap(), ["Alex clicked minecraft:stone at [1, 64, 0] Up", "Alex clicked minecraft:lever at [2, 65, 0] Up"]);
}

#[test]
fn requests_out_of_reach_out_of_the_world_or_malformed_change_nothing() {
    let mut h = two_players(GameMode::Survival);
    // Ten blocks away: the breaker is shown the block, and it stays.
    h.receive(1, &block_actions(&[(Action::PredictBreak, [7, 64, 7])]));
    let sent = h.tick_all();
    assert_eq!(updates(sent.to(1)), [shown([7, 64, 7], STONE), shown([7, 65, 7], AIR)].concat());
    assert!(quiet(sent.to(2)));

    // Above and below the world, at the edge of the numbers, a face that does not exist, air.
    let nowhere = [[0, 400, 0], [0, -70, 0], [i32::MAX, i32::MAX, i32::MAX], [i32::MIN, i32::MIN, i32::MIN]];
    for pos in nowhere {
        h.receive(1, &block_actions(&[(Action::StartBreak, pos), (Action::PredictBreak, pos)]));
        h.receive(1, &click(pos, UP, Some(DIRT)));
        let sent = h.tick_all();
        assert!(quiet(sent.to(1)) && quiet(sent.to(2)), "{pos:?}");
    }
    h.receive(1, &click(NEAR, 9, Some(DIRT)));
    h.receive(1, &click(ABOVE, UP, Some(DIRT)));
    let sent = h.tick_all();
    assert_eq!(updates(sent.to(1)), [shown(NEAR, STONE), shown(ABOVE, AIR), shown([1, 66, 0], AIR)].concat());
    assert!(quiet(sent.to(2)));
    assert_eq!(name(&mut h, [7, 64, 7]), STONE);
}

#[test]
fn handlers_set_no_block_above_the_world_or_in_a_column_that_is_not_loaded() {
    let mut h = Harness::with(ShardConfig { game_mode: GameMode::Creative, spawn: [0.5, 319.0, 0.5], ..config() }, World::with_storage(Arc::new(MemoryStorage::default())));
    h.join_to_radius(1, 2);
    let far = [1000, 64, 1000];
    h.remote.run(move |ctx| assert!(ctx.block(far).is_none() && !ctx.set_block(far, &Block::new("stone")) && !ctx.set_block([0, 320, 0], &Block::new("stone"))));
    h.tick(1);
    // Storage answers on its own threads: wait for the column under the spawn.
    let loaded = Arc::new(Mutex::new(false));
    while !*loaded.lock().unwrap() {
        let out = loaded.clone();
        h.remote.run(move |ctx| *out.lock().unwrap() = ctx.set_block([1, 319, 0], &Block::new("stone")));
        h.tick(1);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(!block(&mut h, [0, 319, 0]).unwrap().name.is_empty() && block(&mut h, [0, 320, 0]).is_none());
}
