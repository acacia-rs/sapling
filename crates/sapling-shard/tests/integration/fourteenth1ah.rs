//! Group AH of the fourteenth wave: the wither's spawn spin (wither/spin.rs) and swoop after a volley
//! (wither/swoop.rs), and a squid pressed along a wall leaving it on its idle pick (creatures/squids.rs);
//! docs/research/wither-lightning-fireworks.md and mob-attacks.md "Squids".

use crate::common;
use crate::fourteenth1ab::pool;

use acacia_proto::packets::{AddEntity, LevelSoundEvent, MoveEntityDelta};
use common::{Harness, all, asking::ask};
use sapling_api::{Difficulty, Effect, PlayerId};

const WITHER: &str = "minecraft:wither";

/// A harness on a stone floor 128 across: a swoop takes a wither 10 to 20 off its prey, past the
/// common 16-across platform, where it would lose sight of its prey and fall.
pub(crate) fn wide_floor() -> Harness {
    let mut level = common::platform();
    let stone = level.block_id("minecraft:stone").unwrap();
    level.fill([-64, 64, -64], [63, 64, 63], stone);
    Harness::with(common::config(), level)
}

#[test]
fn a_charging_wither_spins_ever_faster_and_plays_spawn_each_time_round() {
    // bds-ah1wither-gdb2 (`WitherBoss` 0xbbba9f0): 10 degrees a tick at first, 0.2 more each tick, a global
    // `spawn` each time the yaw comes round past 0; 22 on BDS and on sapling over the ab1wither2 scene.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Normal);
        assert!(ctx.spawn_entity(WITHER, [8.5, 66.0, 0.5]).is_some());
    });
    let (mut yaws, mut sounds) = (Vec::new(), 0);
    for _ in 0..100 {
        yaws.extend(ask(&mut h, |ctx| ctx.entities().into_iter().find(|entity| entity.kind == WITHER).map(|entity| entity.yaw)));
        sounds += all::<LevelSoundEvent>(h.tick_all().to(1)).iter().filter(|sound| sound.sound_id == "spawn").count();
    }
    let turns: Vec<f32> = yaws.windows(2).map(|pair| (pair[0] - pair[1]).rem_euclid(360.0)).collect();
    let quarter = turns.len() / 4;
    let mean = |turns: &[f32]| turns.iter().sum::<f32>() / turns.len().max(1) as f32;
    let (first, last) = (mean(&turns[..quarter]), mean(&turns[turns.len() - quarter..]));
    assert!(turns.len() >= 80 && turns.iter().all(|turn| *turn > 0.0 && *turn < 180.0), "always the one way round ({} yaws, {sounds} sounds): {turns:?}", yaws.len());
    assert!(last > 2.0 * first, "faster: {first} then {last}");
    // Every other tick is an `ask`, whose packets may not come back here.
    assert!((5..=23).contains(&sounds), "{sounds} spawn sounds");
}

#[test]
fn an_unarmoured_wither_swoops_across_after_a_volley_and_its_middle_head_rests_through_it() {
    // bds-ah1wither-gdb1, bds-ae1wither2: 40 ticks after the blue skull it flew to a spot 10 off its prey,
    // and the middle head shot again only 20 ticks after the swoop, on its beat.
    let mut h = wide_floor();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Normal);
        assert!(ctx.spawn_entity(WITHER, [24.5, 66.0, 0.5]).is_some());
        assert!(ctx.give_effect(PlayerId(1), Effect::Resistance, 1000, 4, false));
    });
    let added: Vec<AddEntity> = (0..201).flat_map(|_| all::<AddEntity>(h.tick_all().to(1))).collect();
    let id = added.iter().find(|added| added.entity_type == WITHER).expect("the wither shown").runtime_id;
    let mut at = ask(&mut h, |ctx| ctx.entities().into_iter().find(|entity| entity.kind == WITHER).map(|entity| entity.feet)).expect("the wither");
    let (mut track, mut middle, mut blue) = (Vec::new(), Vec::new(), None);
    for tick in 0..500 {
        let told = h.tick_all();
        for moved in all::<MoveEntityDelta>(told.to(1)).into_iter().filter(|moved| moved.runtime_entity_id == id) {
            at = [moved.x.unwrap_or(at[0]), moved.y.unwrap_or(at[1]), moved.z.unwrap_or(at[2])];
        }
        for skull in all::<AddEntity>(told.to(1)) {
            let from_middle = (skull.position.x - at[0]).hypot(skull.position.z - at[2]) < 0.6;
            match skull.entity_type.as_str() {
                "minecraft:wither_skull_dangerous" if blue.is_none() => blue = Some(tick),
                "minecraft:wither_skull" if from_middle => middle.push(tick),
                _ => {}
            }
        }
        track.push(at);
    }
    let blue = blue.expect("a blue skull");
    assert!(blue + 200 < track.len(), "the blue skull at {blue} came too late to watch the swoop");
    let rest = track[blue + 38];
    let moved = (blue + 40..blue + 160).map(|tick| (track[tick][0] - rest[0]).hypot(track[tick][2] - rest[2])).fold(0.0f32, f32::max);
    assert!(moved > 8.0, "it swooped from {rest:?}: {:?}", track[blue..blue + 160].iter().step_by(10).collect::<Vec<_>>());
    let next = middle.iter().find(|shot| **shot > blue).expect("the middle head shot again");
    assert!(*next >= blue + 70, "the middle head rested through the swoop: blue {blue}, next {next}, middle {middle:?}");
}

#[test]
fn a_squid_pressed_along_a_wall_leaves_it_on_its_idle_pick() {
    // bds-ah1swim gdb: BDS's idle goal is never stopped by the pressed goal, so its pick every 201 ticks
    // takes a squid off a wall it creeps along; sapling's had held it there for 40 s.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        pool(ctx, [10, 10], [4, 4], [60, 64]);
        for at in [[10.5, 61.0, 10.5], [13.5, 61.0, 13.5], [12.5, 62.0, 11.5]] {
            assert!(ctx.spawn_entity("minecraft:squid", at).is_some());
        }
    });
    let mut tracks: Vec<Vec<[f32; 3]>> = Vec::new();
    for _ in 0..2400 {
        let squids: Vec<[f32; 3]> = ask(&mut h, |ctx| ctx.entities().into_iter().filter(|entity| entity.kind == "minecraft:squid").map(|entity| entity.feet).collect());
        tracks.resize(squids.len().max(tracks.len()), Vec::new());
        squids.into_iter().enumerate().for_each(|(squid, feet)| tracks[squid].push(feet));
    }
    let longest = |track: &Vec<[f32; 3]>| {
        let (mut run, mut longest) = (0, 0);
        for pair in track.windows(2) {
            let step = (0..3).map(|axis| (pair[1][axis] - pair[0][axis]).powi(2)).sum::<f32>().sqrt();
            run = if step > 0.0 && step < 0.0016 { run + 1 } else { 0 };
            longest = longest.max(run);
        }
        longest
    };
    let runs: Vec<usize> = tracks.iter().map(longest).collect();
    // A pick into a corner can hold it a second beat; held until it left the wall, runs reached 1380 ticks.
    assert!(runs.iter().all(|run| *run <= 600) && runs.iter().any(|run| *run >= 20), "creeps end on an idle pick: {runs:?}");
}
