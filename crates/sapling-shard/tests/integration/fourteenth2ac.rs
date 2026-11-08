//! Group AC of the fourteenth wave: when a parrot takes off (RandomFlyingGoal::canUse, bds-x2parrot.txt)
//! and a bee at its flower (bds-bees.txt).

use std::sync::{Arc, Mutex};

use crate::common;

use common::Harness;
use sapling_api::{Block, EntityId, GameMode, PlayerId};

const FLOOR: f32 = 65.0;

/// Each entity's feet, a tick at a time for `ticks`.
fn tracks(h: &mut Harness, ids: &[u64], ticks: usize) -> Vec<Vec<[f32; 3]>> {
    let mut tracks = vec![Vec::new(); ids.len()];
    let feet = Arc::new(Mutex::new(Vec::new()));
    for _ in 0..ticks {
        let (out, ids) = (feet.clone(), ids.to_vec());
        h.remote.run(move |ctx| *out.lock().unwrap() = ids.iter().map(|id| ctx.entity(EntityId(*id)).map_or([f32::NAN; 3], |seen| seen.feet)).collect());
        h.tick_all();
        for (track, at) in tracks.iter_mut().zip(feet.lock().unwrap().iter()) {
            track.push(*at);
        }
    }
    tracks
}

/// bds-x2parrot.txt: every takeoff came 7 to 98 ticks after a landing, and each of the three parrots
/// then sat for good: a parrot rolls its 1 in 120 only within 100 ticks of its last flight's end.
#[test]
fn a_parrot_takes_off_only_within_100_ticks_of_its_last_flight() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let parrots = Arc::new(Mutex::new(Vec::new()));
    let out = parrots.clone();
    h.remote.run(move |ctx| {
        ctx.set_game_mode(PlayerId(1), GameMode::Creative);
        ctx.teleport(PlayerId(1), [-6.5, FLOOR, -6.5]);
        *out.lock().unwrap() = [-4.5, -1.5, 1.5, 4.5].map(|x| ctx.summon_as(None, "minecraft:parrot", [x, FLOOR + 2.0, 2.5], 0.0, 0.0, None).expect("a parrot").0).to_vec();
    });
    h.tick_all();
    let ids = parrots.lock().unwrap().clone();
    let (mut takeoffs, mut sat_late) = (0, 0);
    for track in tracks(&mut h, &ids, 3000) {
        let heights: Vec<f32> = track.iter().map(|at| at[1]).collect();
        let mut landed = None;
        for (tick, pair) in heights.windows(2).enumerate() {
            if pair[0] > FLOOR && pair[1] == FLOOR {
                landed = Some(tick + 1);
            }
            if pair[0] == FLOOR && pair[1] > FLOOR {
                let since = tick - landed.expect("it sat first");
                assert!(since < 100, "took off {since} ticks after landing: {:?}", &heights[landed.unwrap()..tick + 2]);
                takeoffs += 1;
            }
        }
        sat_late += usize::from(heights[2000..].iter().all(|y| *y == FLOOR));
    }
    assert!(takeoffs > 0 && sat_late == ids.len(), "{takeoffs} takeoffs, {sat_late} sat for good");
}

/// bds-bees.txt: a bee flew to its flower until its middle came within 1 of the spot over it, then
/// was no longer pushed and came to a stop, still for 95 ticks; one pushed at the spot all along
/// bobbed about it (sapling-bees.txt before).
#[test]
fn a_bee_at_its_flower_drifts_to_a_stop_and_stays_still() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let bee = Arc::new(Mutex::new(0));
    let out = bee.clone();
    h.remote.run(move |ctx| {
        ctx.set_time(6000);
        ctx.set_game_mode(PlayerId(1), GameMode::Creative);
        assert!(ctx.set_game_rule("doMobSpawning", sapling_api::RuleValue::Bool(false)));
        assert!(ctx.set_block([0, 64, 3], &Block::new("grass_block")) && ctx.set_block([0, 65, 3], &Block::new("poppy")));
        *out.lock().unwrap() = ctx.spawn_entity("minecraft:bee", [3.5, 66.0, -1.5]).expect("a bee").0;
    });
    h.tick_all();
    let id = *bee.lock().unwrap();
    let track = tracks(&mut h, &[id], 400).remove(0);
    let steps: Vec<f32> = track.windows(2).map(|pair| (0..3).map(|axis| (pair[1][axis] - pair[0][axis]).powi(2)).sum::<f32>().sqrt()).collect();
    let still = steps.iter().position(|step| *step < 1e-3).expect("it stops");
    let spot = [0.5, 65.25, 3.5];
    let off = (0..3).map(|axis| (track[still][axis] + if axis == 1 { 0.25 } else { 0.0 } - spot[axis]).powi(2)).sum::<f32>().sqrt();
    assert!(off < 1.0, "it stopped {off} off the spot");
    assert!(steps[still..still + 200].iter().all(|step| *step < 1e-3), "it moved again: {:?}", &steps[still..still + 200]);
}
