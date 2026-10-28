//! Sculk catalysts blooming at deaths and spreading the experience as sculk (docs/research/sculk.md,
//! "Catalyst").

use crate::common;

use std::sync::mpsc;

use acacia_proto::RawPacket;
use acacia_proto::packets::{LevelEventGeneric, LevelSoundEvent};
use common::{Harness, all, orbs};
use sapling_api::{Block, PlayerId};

const ALEX: PlayerId = PlayerId(1);
const CATALYST: [i32; 3] = [2, 65, 2];
/// `LevelEventGeneric` ids as sent, zigzagged: bloom 2036, charge 2037.
const BLOOM: u32 = 2036 << 1;
const CHARGE: u32 = 2037 << 1;

fn ready() -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| ctx.teleport(ALEX, [-5.5, 65.0, -5.5]));
    h.remote.run(|ctx| assert!(ctx.update_block(CATALYST, &Block::new("sculk_catalyst"))));
    ticks(&mut h, 4);
    h
}

fn ticks(h: &mut Harness, n: usize) -> Vec<RawPacket> {
    (0..n).flat_map(|_| h.tick_all().to(1).to_vec()).collect()
}

fn properties(h: &mut Harness, pos: [i32; 3]) -> String {
    let (tx, rx) = mpsc::channel();
    h.remote.run(move |ctx| tx.send(ctx.block(pos).map(|block| block.properties).unwrap_or_default()).unwrap());
    h.tick_all();
    rx.recv().unwrap()
}

/// How many of the blocks on and over the platform are named `name`.
fn count(h: &mut Harness, name: &'static str) -> usize {
    let (tx, rx) = mpsc::channel();
    h.remote.run(move |ctx| {
        let cells = (-8..8).flat_map(|x| (64..67).flat_map(move |y| (-8..8).map(move |z| [x, y, z])));
        tx.send(cells.filter(|pos| ctx.block(*pos).is_some_and(|block| block.name == name)).count()).unwrap();
    });
    h.tick_all();
    rx.recv().unwrap()
}

fn events(sent: &[RawPacket], id: u32) -> usize {
    all::<LevelEventGeneric>(sent).iter().filter(|event| event.event_id == id).count()
}

#[test]
fn a_mob_dying_near_a_catalyst_makes_it_bloom_and_spread_sculk() {
    let mut h = ready();
    let (tx, rx) = mpsc::channel();
    h.remote.run(move |ctx| tx.send(ctx.spawn_entity("minecraft:cow", [4.5, 65.0, 4.5]).expect("a cow")).unwrap());
    h.tick_all();
    let cow = rx.recv().unwrap();
    h.remote.run(move |ctx| assert!(ctx.kill_entity(cow)));
    let bloomed = ticks(&mut h, 2);
    assert_eq!(events(&bloomed, BLOOM), 1, "the bloom particles");
    assert!(properties(&mut h, CATALYST).contains("bloom=1"));
    let mut spread = ticks(&mut h, 10);
    assert!(properties(&mut h, CATALYST).contains("bloom=0"), "the bloom lasts 8 ticks");
    spread.extend(bloomed);
    spread.extend(ticks(&mut h, 100));
    assert!(events(&spread, CHARGE) > 0, "the charge is shown moving");
    assert!(all::<LevelSoundEvent>(&spread).iter().any(|sound| sound.sound_id == "block.sculk.spread"));
    assert!(count(&mut h, "minecraft:sculk") > 0, "the stone the charge reached is sculk");
}

#[test]
fn a_player_dying_near_a_catalyst_leaves_its_experience_to_it() {
    let mut h = ready();
    h.remote.run(|ctx| {
        assert!(ctx.give_experience(ALEX, 200));
        ctx.teleport(ALEX, [4.5, 65.0, 4.5]);
    });
    ticks(&mut h, 2);
    h.remote.run(|ctx| assert!(ctx.kill_player(ALEX)));
    let sent = ticks(&mut h, 60);
    assert_eq!(events(&sent, BLOOM), 1);
    assert!(orbs::worths(&sent).is_empty(), "no orbs: the catalyst took the experience");
    ticks(&mut h, 200);
    assert!(count(&mut h, "minecraft:sculk") > 0);
}

#[test]
fn a_death_out_of_range_does_nothing() {
    let mut h = ready();
    let (tx, rx) = mpsc::channel();
    h.remote.run(move |ctx| tx.send(ctx.spawn_entity("minecraft:cow", [-6.5, 65.0, -6.5]).expect("a cow")).unwrap());
    h.tick_all();
    let cow = rx.recv().unwrap();
    h.remote.run(move |ctx| assert!(ctx.kill_entity(cow)));
    let sent = ticks(&mut h, 40);
    assert_eq!(events(&sent, BLOOM), 0);
    assert!(properties(&mut h, CATALYST).contains("bloom=0"));
    assert_eq!(count(&mut h, "minecraft:sculk"), 0);
}
