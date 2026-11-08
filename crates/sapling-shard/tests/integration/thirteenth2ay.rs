//! Group AY: rules read off the Linux BDS under gdb (research/captures/bds-ay2gdb.txt; THIRTEENTH-BRIEF.md).

use crate::common;

use acacia_proto::packets::{LevelEvent, LevelEventEvent, LevelSoundEvent};
use common::raising::feet_of;
use common::shooting::{ALEX, FEET, watch};
use common::{Harness, all};
use sapling_api::{Block, GameMode, RuleValue, ShardCtx};

const EGG: [i32; 3] = [6, 65, 6];

fn hold_night(ctx: &mut ShardCtx<'_>) {
    assert!(ctx.set_game_rule("doDaylightCycle", RuleValue::Bool(false)));
    ctx.set_time(18_000);
}

/// A zombie spawned at `at` with four turtle eggs at [`EGG`] (one egg may break underfoot, 1 in 100 a tick):
/// the crumbles and attack sounds sent till the block is stomped out.
fn stomped(at: [f32; 3]) -> (usize, usize) {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| {
        ctx.set_difficulty(sapling_api::Difficulty::Easy);
        ctx.set_game_mode(sapling_api::PlayerId(1), GameMode::Creative);
        hold_night(ctx);
        assert!(ctx.set_block(EGG, &Block::new("turtle_egg").with("cracked_state=no_cracks,turtle_egg_count=four_egg")));
        assert!(ctx.spawn_entity("minecraft:zombie", at).is_some());
    });
    let sent: Vec<_> = (0..600).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let events = all::<LevelEvent>(&sent);
    assert!(events.iter().any(|event| event.event == LevelEventEvent::ParticleDestroyEgg), "stomped out");
    let crumbles = events.iter().filter(|event| event.event == LevelEventEvent::ParticleBreakingEgg).count();
    let heard = all::<LevelSoundEvent>(&sent).iter().filter(|sound| sound.sound_id == "block.turtle_egg.attack").count();
    (crumbles, heard)
}

/// BDS's `StompEggGoal` counts from reaching the egg (within 1.14 of the cell's corner one up) and crumbles it only
/// standing in its cell, from count 1 till it goes at count 61: a zombie on the egg crumbles it 61 times and is
/// heard at every 6th count from 0; one walking up crumbles it fewer times (50 in bds-av2stomp.txt and bds-ay2gdb.txt),
/// stepping up onto it (body/step.rs).
#[test]
fn a_zombie_stomps_a_turtle_egg_out_61_counts_after_reaching_it() {
    assert_eq!(stomped([6.5, 65.0, 6.5]), (61, 11));
    let (crumbles, _) = stomped([0.5, 65.0, 6.5]);
    assert!((40..61).contains(&crumbles), "{crumbles}");
}

/// BDS's `DropItemForGoal` (bds-ay2gdb.txt): woken in the morning, the cat is put on the floor cell whose
/// squared distance from its owner's eye block (0, 66, 0) is first nearest 25 (24, ring 4's dx 2), (2, 64, 4),
/// at its corner one up.
#[test]
fn a_cat_waking_with_its_owner_in_the_morning_goes_to_its_offering_spot() {
    let (mut h, cat) = crate::thirteenth2ar::cat_on_owners_bed();
    h.remote.run(|ctx| ctx.set_time(24_000));
    watch(&mut h, 1);
    h.remote.run(|ctx| ctx.teleport(ALEX, FEET));
    watch(&mut h, 2);
    let at = feet_of(&mut h, cat);
    assert!((at[0] - 2.0).hypot(at[2] - 4.0) < 0.3 && (at[1] - 65.0).abs() < 0.1, "at its spot: {at:?}");
}
