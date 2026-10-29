//! Egg blocks as BDS showed them (research/captures/bds-eggs.txt; docs/research/mob-life.md).

use crate::common;

use acacia_proto::packets::{AddEntity, LevelEvent, LevelEventEvent, LevelSoundEvent, UpdateBlock};
use common::{Harness, all};
use sapling_api::{Block, RuleValue, ShardCtx};

fn set(ctx: &mut ShardCtx<'_>, pos: [i32; 3], block: Block) {
    assert!(ctx.set_block(pos, &block), "{block:?} at {pos:?}");
}

fn name_at(h: &mut Harness, at: [i32; 3]) -> String {
    let (sent, got) = std::sync::mpsc::channel();
    h.remote.run(move |ctx| sent.send(ctx.block(at).map_or(String::new(), |block| block.name)).unwrap());
    h.tick_all();
    got.recv().unwrap()
}

#[test]
fn hatched_turtles_all_come_out_at_the_cells_corner_and_are_heard_there() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        assert!(ctx.set_game_rule("doDaylightCycle", RuleValue::Bool(false)));
        ctx.set_time(21_500);
        set(ctx, [4, 64, 4], Block::new("sand"));
        set(ctx, [4, 65, 4], Block::new("turtle_egg").with("cracked_state=max_cracked,turtle_egg_count=three_egg"));
    });
    let sent: Vec<_> = (0..1010).flat_map(|_| h.tick(1)).collect();
    let spots: Vec<[f32; 3]> = all::<AddEntity>(&sent).iter().filter(|added| added.entity_type == "minecraft:turtle").map(|added| [added.position.x, added.position.y, added.position.z]).collect();
    assert_eq!(spots.len(), 3);
    assert!(spots.iter().all(|[x, y, z]| (x - 3.88).abs() < 1e-3 && *y == 65.0 && *z == 4.0), "{spots:?}");
    let hatch = all::<LevelSoundEvent>(&sent).into_iter().find(|sound| sound.sound_id == "block.turtle_egg.hatch").expect("heard");
    assert_eq!(([hatch.position.x, hatch.position.y, hatch.position.z], hatch.extra_data), ([4.0, 65.0, 4.0], -1));
}

#[test]
fn a_sniffer_egg_on_moss_sparkles_its_cracked_state_and_hatches_at_the_cells_corner() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        for x in [4, 8] {
            set(ctx, [x, 64, 4], Block::new("moss_block"));
        }
        set(ctx, [4, 65, 4], Block::new("sniffer_egg"));
        set(ctx, [8, 65, 4], Block::new("sniffer_egg").with("cracked_state=max_cracked"));
    });
    let sent: Vec<_> = (0..4400).flat_map(|_| h.tick(1)).collect();
    let sparkles: Vec<u32> = all::<LevelEvent>(&sent).iter().filter(|event| event.event == LevelEventEvent::ParticleTurtleEgg && event.position.x == 4.0).map(|event| event.data as u32).collect();
    let states: Vec<u32> = all::<UpdateBlock>(&sent).iter().filter(|update| update.position.x == 4).map(|update| update.block_runtime_id).collect();
    assert_eq!(sparkles.len(), 2, "placed, then cracked once");
    assert_eq!(Some(&sparkles[1]), states.last(), "the sparkle names the cracked state: {states:?}");
    let born: Vec<[f32; 3]> = all::<AddEntity>(&sent).iter().filter(|added| added.entity_type == "minecraft:sniffer").map(|added| [added.position.x, added.position.y, added.position.z]).collect();
    assert_eq!(born, [[8.0, 65.0, 4.0]]);
}

#[test]
fn frogspawn_breaks_as_its_water_goes_and_is_heard_laid_only_on_water() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let mut sent = Vec::new();
    h.remote.run(|ctx| {
        set(ctx, [4, 64, 4], Block::new("water"));
        set(ctx, [4, 65, 4], Block::new("frog_spawn"));
        set(ctx, [8, 65, 4], Block::new("frog_spawn"));
    });
    (0..2).for_each(|_| sent.extend(h.tick(1)));
    let laid: Vec<f32> = all::<LevelSoundEvent>(&sent).iter().filter(|sound| sound.sound_id == "lay_spawn").map(|sound| sound.position.x).collect();
    assert_eq!(laid, [4.0], "only the one on water");
    h.remote.run(|ctx| assert!(ctx.update_block([4, 64, 4], &Block::new("stone"))));
    (0..3).for_each(|_| _ = h.tick(1));
    assert_eq!(name_at(&mut h, [4, 65, 4]), "minecraft:air");
    assert_eq!(name_at(&mut h, [8, 65, 4]), "minecraft:frog_spawn", "set on stone it waits");
}
