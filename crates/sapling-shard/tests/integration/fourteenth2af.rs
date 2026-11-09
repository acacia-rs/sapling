//! Group AF of the fourteenth wave: babies following a grown one of their kind (FollowParentGoal,
//! bds-af2baby.txt) and a guarding wolf's way round a wall (bds-af2wolf.txt).

use crate::common;

use acacia_proto::packets::EntityEventEventId as Event;
use common::Harness;
use common::asking::ask;
use common::raising::{click_on, showed};
use common::shooting::{ready, spawn, watch};
use sapling_api::{Block, EntityId, GameMode, PlayerId};

const FLOOR: f32 = 65.0;

/// Each entity's feet, a tick at a time for `ticks`.
fn tracks(h: &mut Harness, ids: &[u64], ticks: usize) -> Vec<Vec<[f32; 3]>> {
    let mut tracks = vec![Vec::new(); ids.len()];
    for _ in 0..ticks {
        let ids = ids.to_vec();
        let feet: Vec<[f32; 3]> = ask(h, move |ctx| ids.iter().map(|id| ctx.entity(EntityId(*id)).map_or([f32::NAN; 3], |seen| seen.feet)).collect());
        for (track, at) in tracks.iter_mut().zip(feet) {
            track.push(at);
        }
    }
    tracks
}

fn across(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] - b[0]).hypot(a[2] - b[2])
}

/// bds-af2baby.txt: a baby cow 7 east of a grown one walked straight to it at 0.163 a tick (movement
/// 0.25 times follow_parent's 1.1) and stopped 2.79 off, inside FollowParentGoal's 3; with the grown
/// one 12 off along z, beyond the 8 its search reaches, it stayed put.
#[test]
fn a_baby_cow_walks_to_a_grown_one_and_stops_three_off() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let (mum, baby) = ask(&mut h, |ctx| {
        ctx.set_game_mode(PlayerId(1), GameMode::Creative);
        ctx.teleport(PlayerId(1), [0.5, FLOOR, -12.5]);
        let mum = ctx.summon_as(None, "minecraft:cow", [0.5, FLOOR, 0.5], 0.0, 0.0, Some("minecraft:spawn_adult")).expect("a cow").0;
        let baby = ctx.summon_as(None, "minecraft:cow", [7.5, FLOOR, 0.5], 270.0, 0.0, Some("minecraft:entity_born")).expect("a calf").0;
        (mum, baby)
    });
    let track = tracks(&mut h, &[mum, baby], 120);
    let paces: Vec<f32> = track[1].windows(2).map(|pair| across(pair[0], pair[1])).collect();
    let top = paces.iter().copied().fold(0.0, f32::max);
    assert!((0.15..0.175).contains(&top), "walked at {top} a tick: {paces:?}");
    let off = across(track[0][119], track[1][119]);
    assert!((2.4..3.0).contains(&off), "stopped {off} off");
    assert!(paces[100..].iter().all(|pace| *pace < 1e-3), "it moved on: {:?}", &paces[100..]);

    assert!(ask(&mut h, move |ctx| ctx.move_entity(EntityId(mum), [-6.5, FLOOR, 12.5])));
    let left = tracks(&mut h, &[baby], 60).remove(0);
    assert!(left.windows(2).all(|pair| across(pair[0], pair[1]) < 1e-3), "it followed one out of reach: {left:?}");
}

/// bds-af2wolf.txt: tamed wolves walled off from what their owner fought took it up unseen and walked
/// round the wall's ends to it.
#[test]
fn a_tamed_wolf_walks_round_a_wall_to_what_its_owner_hits() {
    let mut h = ready(GameMode::Survival, &[("minecraft:bone", 64, 0)], 0.0);
    let wolf = spawn(&mut h, "minecraft:wolf", [3.5, FLOOR, 0.5]);
    assert!((0..40).any(|_| showed(&click_on(&mut h, wolf), wolf, Event::TameSuccess)), "tamed");
    let pig = spawn(&mut h, "minecraft:pig", [0.5, FLOOR, -1.5]);
    ask(&mut h, |ctx| {
        let mut wall = (-5..=5).flat_map(|z| (65..=67).map(move |y| [2, y, z]));
        assert!(wall.all(|at| ctx.set_block(at, &Block::new("minecraft:stone"))));
    });
    h.receive(1, &common::attack(pig));
    assert!((0..300).any(|_| showed(&watch(&mut h, 1), wolf, Event::ArmSwing)), "it never bit the pig behind the wall");
}
