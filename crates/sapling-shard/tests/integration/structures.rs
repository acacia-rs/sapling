//! Structure templates saved and loaded, ticking areas and forced chunks, hardcore deaths and the
//! world border, against research/captures/bds-structures.txt, bds-ticking.txt, bds-hardcore.txt.

use crate::common;

use std::sync::Arc;
use std::time::{Duration, Instant};

use acacia_proto::nbt::Value;
use acacia_proto::packets::{Respawn, StartGame, UpdatePlayerGameType};
use acacia_proto::types::{GameMode as WireGameMode, Vec3f};
use common::{Harness, all, config, platform};
use sapling_api::{Block, GameMode, Item, Mirror, PlayerId, Rotation, ShardCtx, StructureAnimation, StructureError, StructureLoad, TickingArea, WorldBorder};
use sapling_world::{Level, MemoryStorage, StructureTemplate, World};

const STEVE: PlayerId = PlayerId(1);

/// A 3x3x3 box on the platform at the origin: a chest facing east holding diamonds, stairs facing
/// east beside it, a log along x at the back and a pig in the middle, as in the BDS drill.
fn build(ctx: &mut ShardCtx<'_>) {
    assert!(ctx.set_block([1, 65, 0], &Block::new("chest").with("minecraft:cardinal_direction=east")));
    assert!(ctx.set_container_item([1, 65, 0], 0, Some(&Item::new("minecraft:diamond", 5))));
    assert!(ctx.set_block([2, 65, 0], &Block::new("oak_stairs").with("upside_down_bit=0,weirdo_direction=0")));
    assert!(ctx.set_block([1, 65, 2], &Block::new("oak_log").with("pillar_axis=x")));
    assert!(ctx.spawn_entity("minecraft:pig", [1.5, 65.0, 1.5]).is_some());
}

fn pigs(ctx: &ShardCtx<'_>) -> Vec<[f32; 3]> {
    ctx.entities().into_iter().filter(|entity| entity.kind == "minecraft:pig").map(|entity| entity.feet).collect()
}

#[test]
fn a_saved_structure_loads_turned_with_what_it_holds() {
    let mut h = Harness::new();
    h.enter(1, "Steve");
    h.remote.run(|ctx| {
        build(ctx);
        assert_eq!(ctx.save_structure("house", [0, 64, 0], [2, 66, 2], true, true, false), Ok(()));
        assert_eq!((ctx.structure_size("house"), ctx.structures()), (Some([3, 3, 3]), vec!["mystructure:house".to_owned()]));
        let turned = StructureLoad { rotation: Rotation::Clockwise90, ..StructureLoad::default() };
        assert_eq!(ctx.load_structure("mystructure:house", [4, 64, 0], &turned), Ok(()));
        // bds-structures.txt: a quarter turn puts the chest at (2, 1, 1) of the box, facing south.
        assert_eq!(ctx.block([6, 65, 1]), Some(Block::new("chest").with("minecraft:cardinal_direction=south")));
        assert_eq!(ctx.block([6, 65, 2]), Some(Block::new("oak_stairs").with("minecraft:corner=none,upside_down_bit=0,weirdo_direction=2")));
        assert_eq!(ctx.block([4, 65, 1]), Some(Block::new("oak_log").with("pillar_axis=z")));
        assert_eq!(ctx.block([5, 64, 1]), Some(Block::new("stone")));
        let chest = ctx.block_entity([6, 65, 1]).and_then(|data| data.get::<Value>().cloned()).expect("the chest's items came along");
        assert_eq!((chest.get("x"), chest.get("z"), chest.get("forceunpair")), (Some(&Value::Int(6)), Some(&Value::Int(1)), Some(&Value::Byte(1))));
        assert!(matches!(chest.get("Items"), Some(Value::List(items)) if items.items.len() == 1));
        let mut feet = pigs(ctx);
        feet.sort_by(|a, b| a[0].total_cmp(&b[0]));
        assert_eq!(feet, [[1.5, 65.0, 1.5], [5.5, 65.0, 1.5]]);
    });
    h.tick_all();
}

/// bds-structanim.txt: `/structure load … layer_by_layer 2` put its three layers in at ≈14, 27, 40 ticks.
#[test]
fn an_animated_load_puts_a_layer_each_third_of_its_time() {
    let mut h = Harness::new();
    h.enter(1, "Steve");
    h.remote.run(|ctx| {
        build(ctx);
        assert_eq!(ctx.save_structure("house", [0, 64, 0], [2, 66, 2], false, true, false), Ok(()));
        assert_eq!(ctx.load_structure_animated("house", [0, 70, 0], &StructureLoad::default(), StructureAnimation::LayerByLayer, 2.0), Ok(()));
        assert!(ctx.block([0, 70, 0]).is_some_and(|block| block.is_air()), "nothing at once");
    });
    (0..18).for_each(|_| drop(h.tick_all()));
    h.remote.run(|ctx| assert_eq!((ctx.block([0, 70, 0]), ctx.block([1, 71, 0]).is_some_and(|block| block.is_air())), (Some(Block::new("stone")), true)));
    (0..25).for_each(|_| drop(h.tick_all()));
    h.remote.run(|ctx| assert_eq!(ctx.block([1, 71, 0]), Some(Block::new("chest").with("minecraft:cardinal_direction=east"))));
    h.tick_all();
}

#[test]
fn a_template_reads_back_mirrors_and_loads_part_of_itself_by_seed() {
    let mut h = Harness::new();
    h.enter(1, "Steve");
    h.remote.run(|ctx| {
        build(ctx);
        assert_eq!(ctx.save_structure("house", [0, 64, 0], [2, 66, 2], false, true, true), Ok(()));
        let template = StructureTemplate::decode(&ctx.structure_template("house").unwrap()).unwrap();
        assert_eq!((template.size, template.origin, template.block_entities.len(), template.entities.len()), ([3, 3, 3], [0, 64, 0], 1, 0));
        assert_eq!(ctx.set_structure_template("copy", vec![1, 2, 3], false), Err(StructureError::Unreadable));
        assert_eq!(ctx.set_structure_template("copy", template.encode(), false), Ok(()));
        // Mirrored across z (`z`), the chest faces west and the stairs too.
        let mirrored = StructureLoad { mirror: Mirror::Z, ..StructureLoad::default() };
        assert_eq!(ctx.load_structure("copy", [-6, 64, 0], &mirrored), Ok(()));
        assert_eq!(ctx.block([-5, 65, 0]), Some(Block::new("chest").with("minecraft:cardinal_direction=west")));
        assert_eq!(ctx.block([-6, 65, 0]), Some(Block::new("oak_stairs").with("minecraft:corner=none,upside_down_bit=0,weirdo_direction=1")));
        // Half the cells by the same seed twice are the same half.
        let half = StructureLoad { integrity: 0.5, seed: "abc".into(), ..StructureLoad::default() };
        assert_eq!((ctx.load_structure("copy", [-6, 64, 4], &half), ctx.load_structure("copy", [-6, 64, -4], &half)), (Ok(()), Ok(())));
        let column = |ctx: &ShardCtx<'_>, z0: i32| (0..3).flat_map(|x| (64..67).flat_map(move |y| (0..3).map(move |z| [x - 6, y, z + z0]))).map(|at| ctx.block(at).is_some_and(|block| !block.is_air())).collect::<Vec<_>>();
        let (first, second) = (column(ctx, 4), column(ctx, -4));
        assert_eq!(first, second, "the same seed places the same cells");
        assert!(first.iter().any(|placed| *placed) && first.iter().any(|placed| !*placed), "{first:?}");
    });
    h.tick_all();
}

#[test]
fn a_deleted_structure_is_gone_and_a_missed_name_deletes_once() {
    let mut h = Harness::new();
    h.enter(1, "Steve");
    h.remote.run(|ctx| {
        assert_eq!(ctx.save_structure("s1", [0, 64, 0], [1, 64, 1], true, true, true), Ok(()));
        assert!(ctx.delete_structure("s1") && !ctx.delete_structure("s1"));
        assert_eq!(ctx.load_structure("s1", [0, 70, 0], &StructureLoad::default()), Err(StructureError::NotFound));
        // BDS answered a delete of a name a load had missed as done, the first time.
        assert!(ctx.delete_structure("s1") && !ctx.delete_structure("s1"));
    });
    h.tick_all();
}

/// Ticks until `done` holds, at most ten seconds.
fn tick_until(h: &mut Harness, done: impl Fn(&mut ShardCtx<'_>) -> bool + Send + Sync + 'static) {
    let done = Arc::new(done);
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let (sender, answer) = std::sync::mpsc::channel();
        let check = done.clone();
        h.remote.run(move |ctx| sender.send(check(ctx)).unwrap());
        h.tick_all();
        if answer.recv().unwrap() {
            return;
        }
        assert!(Instant::now() < deadline, "never came to be");
    }
}

#[test]
fn ticking_areas_and_forced_chunks_stay_loaded_with_nobody_near() {
    let mut h = Harness::with(config(), World::with_storage(Arc::new(MemoryStorage::default())));
    common::stored::enter_stored(&mut h, 1);
    h.remote.run(|ctx| {
        assert!(ctx.block([400, 64, 400]).is_none());
        assert_eq!(ctx.save_structure("far", [400, 64, 400], [401, 64, 401], true, true, false), Err(StructureError::Unloaded));
        assert!(ctx.add_ticking_area(TickingArea { name: "farm".into(), min: [25, 25], max: [26, 26], circle: false, preload: false }));
        ctx.force_chunk([-30, -30], true);
        assert_eq!((ctx.ticking_areas().len(), ctx.forced_chunks()), (1, vec![[-30, -30]]));
    });
    tick_until(&mut h, |ctx| ctx.block([400, 64, 400]).is_some() && ctx.block([431, 64, 431]).is_some() && ctx.block([-480, 64, -480]).is_some());
    (0..120).for_each(|_| drop(h.tick_all()));
    h.remote.run(|ctx| {
        assert!(ctx.block([400, 64, 400]).is_some() && ctx.block([-480, 64, -480]).is_some(), "an unload sweep left them");
        assert_eq!(ctx.remove_ticking_areas(&|area| area.name == "farm").len(), 1);
        ctx.force_chunk([-30, -30], false);
    });
    (0..120).for_each(|_| drop(h.tick_all()));
    h.remote.run(|ctx| assert!(ctx.block([400, 64, 400]).is_none() && ctx.block([-480, 64, -480]).is_none()));
    h.tick_all();
}

#[test]
fn a_hardcore_world_says_so_and_its_dead_rise_as_spectators() {
    let mut world = platform();
    *world.level_mut() = Level { hardcore: true, ..Level::default() };
    let mut h = Harness::with(config(), world);
    h.enter(1, "Steve");
    h.remote.run(|ctx| {
        assert!(ctx.is_hardcore());
        ctx.set_game_mode(STEVE, GameMode::Survival);
        assert!(ctx.kill_player(STEVE));
    });
    h.tick_all();
    h.receive(1, &Respawn { position: Vec3f { x: 0.0, y: 0.0, z: 0.0 }, state: 2, runtime_entity_id: 0 });
    let risen = h.tick_all();
    let modes: Vec<WireGameMode> = all::<UpdatePlayerGameType>(risen.to(1)).into_iter().map(|update| update.gamemode).collect();
    assert_eq!(modes, [WireGameMode::Spectator]);
    h.remote.run(|ctx| assert_eq!(ctx.game_mode(STEVE), Some(GameMode::Spectator)));
    h.tick_all();
}

#[test]
fn start_game_tells_a_hardcore_world() {
    let mut world = platform();
    *world.level_mut() = Level { hardcore: true, ..Level::default() };
    let mut h = Harness::with(config(), world);
    let sent = h.join_to_radius(1, 2);
    assert!(all::<StartGame>(&sent)[0].hardcore);
    let mut h = Harness::new();
    assert!(!all::<StartGame>(&h.join_to_radius(1, 2))[0].hardcore);
}

#[test]
fn a_player_past_the_border_is_put_back_inside() {
    let mut h = Harness::new();
    h.enter(1, "Steve");
    h.remote.run(|ctx| {
        assert_eq!(ctx.world_border(), None);
        ctx.set_world_border(Some(WorldBorder { centre: [0.0, 0.0], radius: 5.0 }));
        ctx.teleport(STEVE, [7.5, 65.0, -9.0]);
    });
    h.tick_all();
    h.tick_all();
    h.remote.run(|ctx| {
        let feet = ctx.position(STEVE).unwrap().feet;
        assert_eq!([feet[0], feet[2]], [4.5, -4.5]);
        ctx.set_world_border(None);
        assert_eq!(ctx.world_border(), None);
    });
    h.tick_all();
}
