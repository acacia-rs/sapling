//! Group W of the fourteenth wave: what pops and what `/kill`ed items send to sculk
//! (research/captures/bds-w1pops.txt), and squids out of water and fleeing to the surface
//! (bds-f14esquid.txt; creatures/squids.rs).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::nbt::Value;
use acacia_proto::packets::{AddEntity, MoveEntityDelta};
use common::{Harness, all, asking::ask, attack};
use sapling_api::{Block, Difficulty, Item};

const SENSOR: [i32; 3] = [4, 65, 0];

/// Alex by a sculk sensor, with `blocks` set quietly.
fn by_a_sensor(blocks: Vec<([i32; 3], Block)>) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| {
        assert!(ctx.update_block(SENSOR, &Block::new("minecraft:sculk_sensor")));
        for (pos, block) in &blocks {
            assert!(ctx.update_block(*pos, block), "{block:?}");
        }
    });
    (0..4).for_each(|_| drop(h.tick_all()));
    h
}

/// The event type the sensor kept, if it heard one.
fn heard(h: &mut Harness) -> Option<i32> {
    let nbt = ask(h, |ctx| ctx.block_entity(SENSOR).and_then(|data| data.get::<Value>().cloned())).expect("a block actor");
    match nbt.get("VibrationListener").and_then(|listener| listener.get("event")) {
        Some(Value::Int(event)) => Some(*event),
        _ => None,
    }
}

#[test]
fn a_door_losing_its_lower_half_and_a_torch_its_block_are_heard_as_blocks_destroyed() {
    let door = |upper: u8| Block::new("minecraft:wooden_door").with(format!("door_hinge_bit=0,minecraft:cardinal_direction=south,open_bit=0,upper_block_bit={upper}"));
    let mut h = by_a_sensor(vec![([0, 65, 0], door(0)), ([0, 66, 0], door(1))]);
    // As `/setblock`: the cell's neighbours hear of it.
    h.remote.run(|ctx| assert!(ctx.update_block([0, 65, 0], &Block::new("minecraft:air"))));
    (0..10).for_each(|_| drop(h.tick_all()));
    assert_eq!(ask(&mut h, |ctx| ctx.block([0, 66, 0]).map(|block| block.name)), Some("minecraft:air".into()), "the upper half popped");
    assert_eq!(heard(&mut h), Some(6), "block_destroy");
    let mut h = by_a_sensor(vec![([0, 65, 2], Block::new("minecraft:stone")), ([0, 66, 2], Block::new("minecraft:torch").with("torch_facing_direction=top"))]);
    h.remote.run(|ctx| assert!(ctx.update_block([0, 65, 2], &Block::new("minecraft:air"))));
    (0..10).for_each(|_| drop(h.tick_all()));
    assert_eq!(heard(&mut h), Some(6), "block_destroy");
}

#[test]
fn a_killed_item_is_heard_as_a_death() {
    let mut h = by_a_sensor(Vec::new());
    // Out of Alex's reach to pick it up.
    h.remote.run(|ctx| assert!(ctx.drop_item(&Item::new("minecraft:cobblestone", 1), [3.5, 65.0, 3.5])));
    (0..40).for_each(|_| drop(h.tick_all()));
    assert_eq!(heard(&mut h), None, "nothing yet");
    let item = ask(&mut h, |ctx| ctx.entities().into_iter().find(|entity| entity.kind == "minecraft:item").map(|entity| entity.id)).expect("the item");
    h.remote.run(move |ctx| assert!(ctx.kill_entity(item)));
    (0..10).for_each(|_| drop(h.tick_all()));
    assert_eq!(heard(&mut h), Some(18), "entity_die, frequency 15");
}

/// A squid put at `at` on the test world: the pitches it is told at over `ticks`, in degrees, and
/// each told move (NaN for an axis not told).
fn squid_pitches(at: [f32; 3], ticks: usize) -> (Vec<f32>, Vec<[f32; 3]>) {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| assert!(ctx.spawn_entity("minecraft:squid", at).is_some()));
    let (mut id, mut pitches, mut places) = (None, Vec::new(), Vec::new());
    for _ in 0..ticks {
        let sent = h.tick_all().to(1).to_vec();
        id = id.or(all::<AddEntity>(&sent).iter().find(|added| added.entity_type == "minecraft:squid").map(|added| added.runtime_id));
        for moved in all::<MoveEntityDelta>(&sent).into_iter().filter(|moved| Some(moved.runtime_entity_id) == id) {
            pitches.extend(moved.rot_x.map(|byte| byte as i8 as f32 * 360.0 / 256.0));
            places.push([moved.x, moved.y, moved.z].map(|c| c.unwrap_or(f32::NAN)));
        }
    }
    (pitches, places)
}

#[test]
fn a_squid_on_the_ground_stays_put_and_turns_to_a_random_way_pitched_63_to_117_down() {
    // bds-f14esquid.txt's squid on the tray: no move, its pitch settling at -80 a tenth a tick.
    let (pitches, places) = squid_pitches([4.5, 65.0, 4.5], 80);
    let last = *pitches.last().expect("it turned");
    assert!((-117.0..=-63.0).contains(&last), "{pitches:?}");
    assert!(places.iter().all(|[x, _, z]| x.is_nan() && z.is_nan()), "it is not moved across: {places:?}");
}

#[test]
fn a_squid_falling_through_air_turns_head_down_to_dive() {
    // Off the platform: `SquidDiveGoal` aims it down, (x, -1, z) with x and z under a half.
    let (pitches, _) = squid_pitches([20.5, 70.0, 0.5], 40);
    assert!(pitches.last().is_some_and(|pitch| *pitch < -120.0 || *pitch > 170.0), "{pitches:?}");
}

#[test]
fn a_squid_fleeing_up_past_its_band_keeps_its_way_across() {
    // bds-f14esquid.txt: at the surface the flight's climb went to 0 and its way across stayed.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.receive(1, &common::input_at([0.5, 61.0, 2.5]));
    drop(h.tick_all());
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Normal);
        for x in -3_i32..=4 {
            for z in 1_i32..=12 {
                for y in 59_i32..=66 {
                    let rim = x == -3 || x == 4 || z == 1 || z == 12 || y == 59;
                    assert!(ctx.set_block([x, y, z], &Block::new(if rim { "glass" } else { "water" })));
                }
            }
        }
        assert!(ctx.spawn_entity("minecraft:squid", [0.5, 64.5, 5.0]).is_some());
    });
    let squid = (0..2).flat_map(|_| all::<AddEntity>(h.tick_all().to(1))).find(|added| added.entity_type == "minecraft:squid").expect("shown").runtime_id;
    ask(&mut h, |_| ());
    h.receive(1, &attack(squid));
    let ticks: Vec<Vec<RawPacket>> = (0..30).map(|_| h.tick_all().to(1).to_vec()).collect();
    let z: Vec<f32> = ticks.iter().filter_map(|sent| all::<MoveEntityDelta>(sent).into_iter().filter(|moved| moved.runtime_entity_id == squid).find_map(|moved| moved.z)).collect();
    let late = (z[z.len() - 1] - z[z.len() - 11]) / 10.0;
    assert!(late > 0.05, "it keeps fleeing along z at {late} a tick: {z:?}");
}
