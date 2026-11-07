//! Group AL: what a `/fill`'s neighbours hear, once every cell is written (docs/research/block-rules.md,
//! "`/fill` neighbour updates"; research/captures/bds-ai1liquid.txt, bds-al2fill.txt).

use crate::common;

use acacia_proto::packets::LevelSoundEvent;
use common::asking::ask;
use common::{Harness, all};
use sapling_api::{Block, Copied};

/// A stone floor at y 64 under open air, x 0 to 8, z -2 to 4.
fn floored() -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    ask(&mut h, |ctx| {
        for x in 0..=8 {
            for z in -2..=4 {
                ctx.set_block([x, 64, z], &Block::new("stone"));
                (65..=67).for_each(|y| _ = ctx.set_block([x, y, z], &Block::air()));
            }
        }
    });
    h
}

fn name_at(h: &mut Harness, pos: [i32; 3]) -> String {
    ask(h, move |ctx| ctx.block(pos).map(|block| block.name).unwrap_or_default())
}

/// The states at z 1 beside water put at x 2 to 4, z 0 or 2: depth 1 only where that cell ran itself
/// (a neighbour's run reaches it at 2).
fn ran_one_off(h: &mut Harness) -> Vec<String> {
    (2..=4).map(|x| ask(h, move |ctx| ctx.block([x, 65, 1]).map(|block| block.properties).unwrap_or_default())).collect()
}

/// BDS's three-cell water line flowed from all three (bds-ai1liquid.txt); sapling's last stood still
/// while each cell's neighbours heard of it as it was written.
#[test]
fn water_filled_in_a_line_flows_from_every_cell() {
    let mut h = floored();
    ask(&mut h, |ctx| ctx.batch_blocks(|ctx| (2..=4).for_each(|x| assert!(ctx.update_block([x, 65, 0], &Block::new("water"))))));
    for _ in 0..15 {
        h.tick_all();
    }
    assert_eq!(ran_one_off(&mut h), ["liquid_depth=1"; 3], "each filled cell ran");
}

/// BDS's three water cells cloned out of a glass ring flowed from all three (bds-al2fill.txt).
#[test]
fn water_cloned_in_a_line_flows_from_every_cell() {
    let mut h = floored();
    ask(&mut h, |ctx| {
        for x in 1..=5 {
            (-3..=-1).for_each(|z| _ = ctx.set_block([x, 65, z], &Block::new("glass")));
        }
        (2..=4).for_each(|x| _ = ctx.set_block([x, 65, -2], &Block::new("water")));
    });
    ask(&mut h, |ctx| {
        let copies: Vec<Copied> = (2..=4).map(|x| Copied { from: [x, 65, -2], to: [x, 65, 2], block: Block::new("water"), entity: None }).collect();
        ctx.copy_blocks(&[], &copies);
    });
    for _ in 0..15 {
        h.tick_all();
    }
    assert_eq!(ran_one_off(&mut h), ["liquid_depth=1"; 3], "each cloned cell ran");
}

/// A geyser over lava rumbles `geyser_continuous_eruption_active` 41 ticks after its start's sound, then
/// every 40, from the top of its water (bds-ag2lava.txt, bds-ag2geyser.txt: -23.0 over sulfur at -25).
#[test]
fn a_continuous_geyser_rumbles_41_ticks_after_it_starts_then_every_40() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    ask(&mut h, |ctx| {
        for y in [64, 66] {
            [[3, y, 0], [5, y, 0], [4, y, -1], [4, y, 1]].iter().for_each(|at| _ = ctx.set_block(*at, &Block::new("glass")));
        }
        (66..=68).for_each(|y| _ = ctx.set_block([4, y, 0], &Block::air()));
        ctx.set_block([4, 64, 0], &Block::new("lava"));
        assert!(ctx.update_block([4, 65, 0], &Block { name: "minecraft:potent_sulfur".into(), properties: "potent_sulfur_state=dormant".into() }));
    });
    h.remote.run(|ctx| assert!(ctx.update_block([4, 66, 0], &Block::new("water"))));
    let mut heard = Vec::new();
    for tick in 0..90 {
        let sent = h.tick_all();
        for sound in all::<LevelSoundEvent>(sent.to(1)).into_iter().filter(|sound| sound.sound_id.starts_with("geyser_continuous")) {
            heard.push((tick, sound.sound_id, sound.position.y));
        }
    }
    let start = "geyser_continuous_eruption_start".to_string();
    let active = "geyser_continuous_eruption_active".to_string();
    assert_eq!(heard, [(0, start, 65.5), (41, active.clone(), 67.0), (81, active, 67.0)]);
}

/// A geyser that water set over it starts flies its vibration the tick after its start's sound
/// (bds-ag2lava.txt: 4054 at 20127 after the sound at 20077).
#[test]
fn a_geyser_a_command_starts_flies_its_vibration_a_tick_after_its_sound() {
    use acacia_proto::packets::LevelEventGeneric;
    let mut h = Harness::new();
    h.enter(1, "Alex");
    ask(&mut h, |ctx| {
        for y in [64, 66] {
            [[3, y, 0], [5, y, 0], [4, y, -1], [4, y, 1]].iter().for_each(|at| _ = ctx.set_block(*at, &Block::new("glass")));
        }
        (66..=68).for_each(|y| _ = ctx.set_block([4, y, 0], &Block::air()));
        ctx.set_block([4, 64, 0], &Block::new("lava"));
        ctx.set_block([4, 65, 4], &Block::new("sculk_sensor"));
        assert!(ctx.update_block([4, 65, 0], &Block { name: "minecraft:potent_sulfur".into(), properties: "potent_sulfur_state=dormant".into() }));
    });
    (0..80).for_each(|_| _ = h.tick_all());
    h.remote.run(|ctx| assert!(ctx.update_block([4, 66, 0], &Block::new("water"))));
    let mut heard = Vec::new();
    for tick in 0..4 {
        let sent = h.tick_all();
        if all::<LevelSoundEvent>(sent.to(1)).iter().any(|sound| sound.sound_id == "geyser_continuous_eruption_start") {
            heard.push((tick, "start"));
        }
        if all::<LevelEventGeneric>(sent.to(1)).iter().any(|event| event.event_id == 2027 << 1) {
            heard.push((tick, "flight"));
        }
    }
    assert_eq!(heard, [(0, "start"), (1, "flight")]);
}

/// One cell set alone wakes only its neighbours: water set by itself stands (bds-ai1liquid.txt).
#[test]
fn water_filled_in_one_cell_stands() {
    let mut h = floored();
    ask(&mut h, |ctx| ctx.batch_blocks(|ctx| assert!(ctx.update_block([4, 65, 0], &Block::new("water")))));
    for _ in 0..15 {
        h.tick_all();
    }
    assert_eq!(name_at(&mut h, [4, 65, 1]), "minecraft:air");
}
