//! Mobs born of blocks (blocks/eggs.rs, creatures/hatching.rs; docs/research/mob-life.md): turtle
//! eggs crack, hatch and are trodden.

use crate::common;

use acacia_proto::packets::{AddEntity, LevelSoundEvent};
use acacia_proto::types::MetadataFlags1;
use common::raising::has_flag;
use common::stored::{enter_stored, floor_with};
use common::{Harness, all, config};
use sapling_api::{Block, RuleValue, ShardCtx};
use sapling_world::{ChunkPos, World, WorldStorage};

fn set(ctx: &mut ShardCtx<'_>, pos: [i32; 3], block: Block) {
    assert!(ctx.set_block(pos, &block), "{block:?} at {pos:?}");
}

fn names(h: &mut Harness, cells: Vec<[i32; 3]>) -> Vec<String> {
    let (sent, got) = std::sync::mpsc::channel();
    h.remote.run(move |ctx| sent.send(cells.iter().map(|at| ctx.block(*at).map_or(String::new(), |block| format!("{}[{}]", block.name, block.properties))).collect::<Vec<_>>()).unwrap());
    h.tick(1);
    got.recv().unwrap()
}

/// Stops the clock at `time`: 21500 is dawn to a turtle egg.
fn hold_time(ctx: &mut ShardCtx<'_>, time: i64) {
    assert!(ctx.set_game_rule("doDaylightCycle", RuleValue::Bool(false)));
    ctx.set_time(time);
}

fn turtle_egg(cracks: &str, count: &str) -> Block {
    Block::new("turtle_egg").with(&format!("cracked_state={cracks},turtle_egg_count={count}"))
}

#[test]
fn a_cracked_turtle_egg_on_sand_hatches_at_dawn_into_babies_at_home() {
    let storage = floor_with(Vec::new());
    let mut h = Harness::with(config(), World::with_storage(storage.clone()));
    let mut sent = enter_stored(&mut h, 1);
    h.remote.run(|ctx| {
        hold_time(ctx, 21_500);
        set(ctx, [4, 64, 4], Block::new("sand"));
        set(ctx, [4, 65, 4], turtle_egg("max_cracked", "two_egg"));
    });
    sent.extend((0..1010).flat_map(|_| h.tick(1)));
    assert_eq!(names(&mut h, vec![[4, 65, 4]]), ["minecraft:air[]"]);
    let turtles: Vec<u64> = all::<AddEntity>(&sent).into_iter().filter(|added| added.entity_type == "minecraft:turtle").map(|added| added.runtime_id).collect();
    assert_eq!(turtles.len(), 2, "one turtle an egg");
    assert!(turtles.iter().all(|id| has_flag(&sent, *id, MetadataFlags1::BABY) == Some(true)), "babies");
    assert!(all::<LevelSoundEvent>(&sent).iter().any(|sound| sound.sound_id == "block.turtle_egg.hatch"));
    h.shard.save_worlds();
    let column = storage.load_column(ChunkPos::new(0, 0), 0).unwrap().unwrap();
    let stored = sapling_world::stored_actors(&column.actors).unwrap();
    let homes: Vec<_> = stored.iter().filter(|(actor, ..)| actor.as_ref().is_some_and(|actor| actor.identifier == "minecraft:turtle")).map(|(.., compound)| sapling_world::actor_home(compound)).collect();
    assert_eq!(homes, vec![Some(([4.0, 65.0, 4.0], 0)); 2]);
}

#[test]
fn a_turtle_egg_cracks_at_its_first_tick_on_sand_and_never_off_it() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        hold_time(ctx, 21_500);
        set(ctx, [4, 64, 4], Block::new("sand"));
        set(ctx, [4, 65, 4], turtle_egg("no_cracks", "one_egg"));
        set(ctx, [6, 65, 4], turtle_egg("no_cracks", "one_egg"));
    });
    (0..990).for_each(|_| _ = h.tick(1));
    let before = names(&mut h, vec![[4, 65, 4]]);
    assert!(before[0].contains("no_cracks"), "the first tick is 1000 ticks on: {before:?}");
    (0..20).for_each(|_| _ = h.tick(1));
    let after = names(&mut h, vec![[4, 65, 4], [6, 65, 4]]);
    assert!(after[0].contains("cracked_state=cracked"), "{after:?}");
    assert!(after[1].contains("no_cracks"), "off sand: {after:?}");
}

#[test]
fn a_pig_on_turtle_eggs_breaks_them_and_a_turtle_does_not() {
    let left = |kind: &'static str| {
        let mut h = Harness::new();
        h.enter(1, "Alex");
        h.remote.run(move |ctx| {
            assert!(ctx.set_game_rule("sapling:mobAi", RuleValue::Bool(false)));
            set(ctx, [4, 65, 4], turtle_egg("no_cracks", "four_egg"));
            assert!(ctx.spawn_entity(kind, [4.5, 65.5, 4.5]).is_some());
        });
        (0..800).for_each(|_| _ = h.tick(1));
        names(&mut h, vec![[4, 65, 4]]).remove(0)
    };
    let (pig, turtle) = (left("minecraft:pig"), left("minecraft:turtle"));
    assert!(!pig.contains("four_egg"), "{pig}");
    assert!(turtle.contains("four_egg"), "{turtle}");
}
