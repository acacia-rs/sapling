//! Group AA of the fourteenth wave: the parrot's flight (research/captures/bds-x2parrot.txt).

use std::sync::{Arc, Mutex};

use crate::common;

use acacia_proto::types::MetadataFlags1;
use common::asking::ask;
use common::raising::has_flag;
use common::shooting::{ALEX, ready};
use sapling_api::{EntityId, GameMode};

const FLOOR: f32 = 65.0;

/// bds-x2parrot.txt: summoned 2 over the floor a parrot falls as a chicken (0.0784, then to 0.114 a
/// tick), lands and sits; it takes off without gravity, rising 0.02, 0.42 and 1.00125 over 1, 6 and 11
/// ticks, never above ~4.2 over the floor, and falls again once it stops flying.
#[test]
fn a_parrot_falls_slowly_sits_and_takes_off_without_gravity() {
    let mut h = ready(GameMode::Creative, &[], 0.0);
    let parrot = ask(&mut h, |ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        ctx.teleport(ALEX, [-6.5, FLOOR, -6.5]);
        ctx.summon_as(None, "minecraft:parrot", [2.5, FLOOR + 2.0, 2.5], 0.0, 0.0, None).expect("a parrot")
    });
    let mut heights = vec![FLOOR + 2.0];
    let mut flags = Vec::new();
    let feet = Arc::new(Mutex::new(0.0));
    for _ in 0..2000 {
        let out = feet.clone();
        h.remote.run(move |ctx| *out.lock().unwrap() = ctx.entity(EntityId(parrot.0)).expect("the parrot").feet[1]);
        let sent = h.tick_all().to(1).to_vec();
        flags.push(has_flag(&sent, parrot.0, MetadataFlags1::AFFECTED_BY_GRAVITY));
        heights.push(*feet.lock().unwrap());
    }
    let start = heights.windows(2).position(|pair| pair[1] < pair[0]).expect("it falls");
    let falls: Vec<f32> = heights[start..].windows(2).take_while(|pair| pair[1] < pair[0]).map(|pair| pair[1] - pair[0]).collect();
    assert!((falls[0] + 0.0784).abs() < 1e-4, "{falls:?}");
    assert!(falls.iter().all(|fall| *fall > -0.1143) && falls[falls.len() - 2] < -0.114,"{falls:?}");
    assert_eq!(heights[start + falls.len()], FLOOR, "it lands: {:?}", &heights[..start + falls.len() + 1]);
    let sits = heights.iter().position(|y| *y == FLOOR).unwrap();
    let off = sits + heights[sits..].iter().position(|y| *y > FLOOR).expect("it takes off");
    for (ticks, over) in [(1, 0.02), (6, 0.42), (11, 1.00125)] {
        assert!((heights[off - 1 + ticks] - FLOOR - over).abs() < 1e-3, "{ticks}: {:?}", &heights[off - 1..off + 12]);
    }
    assert_eq!(flags[off - 2..off + 1].iter().flatten().last(), Some(&false), "aloft, no gravity shown");
    assert!(heights.iter().all(|y| *y < FLOOR + 5.5), "{:?}", heights.iter().cloned().fold(0.0, f32::max));
    let back = off + flags[off..].iter().position(|told| *told == Some(true)).expect("gravity back");
    assert!(heights[back..].iter().any(|y| *y == FLOOR), "it lands again");
}

/// bds-aa2spear.txt: a zombie given an iron spear under 10 off (7 here, on the test platform) raises it (`ACTION`) and walks through its
/// prey, hitting it from 2.5 off for 3.5 on easy (3 + floor(2.28 b/s × 0.95), halved plus 1) with a
/// throw of 0.4 away and up and `KineticDamageDealt`, never swinging; it goes on past and turns back.
#[test]
fn a_spear_zombie_charges_through_its_prey_and_hits_by_its_speed() {
    use acacia_proto::packets::{EntityEvent, EntityEventEventId, SetEntityMotion};
    use common::all;
    use sapling_api::{Difficulty, Item, MobSlot};
    let mut h = ready(GameMode::Survival, &[], 0.0);
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_time(18000);
        ctx.set_game_mode(common::shooting::STEVE, GameMode::Creative);
    });
    let zombie = common::shooting::spawn(&mut h, "minecraft:zombie", [0.5, FLOOR, 7.5]);
    ask(&mut h, move |ctx| assert_eq!(ctx.set_mob_item(EntityId(zombie), MobSlot::MainHand, Some(&Item::new("minecraft:iron_spear", 1))), Ok(())));
    let (mut raised, mut events, mut swings, mut throws, mut healths, mut nearest) = (None, Vec::new(), 0, Vec::new(), Vec::new(), f32::MAX);
    let feet = Arc::new(Mutex::new([0.0; 3]));
    for tick in 0..300 {
        let out = feet.clone();
        h.remote.run(move |ctx| *out.lock().unwrap() = ctx.entity(EntityId(zombie)).expect("the zombie").feet);
        let sent = h.tick_all().to(1).to_vec();
        if raised.is_none() && has_flag(&sent, zombie, MetadataFlags1::ACTION) == Some(true) {
            raised = Some(tick);
        }
        let mine = |event: &&EntityEvent| event.runtime_entity_id == zombie;
        events.extend(all::<EntityEvent>(&sent).iter().filter(mine).filter(|event| event.event_id == EntityEventEventId::KineticDamageDealt).map(|_| tick));
        swings += all::<EntityEvent>(&sent).iter().filter(mine).filter(|event| event.event_id == EntityEventEventId::ArmSwing).count();
        throws.extend(all::<SetEntityMotion>(&sent).into_iter().map(|motion| [motion.velocity.x, motion.velocity.y, motion.velocity.z]));
        healths.extend(common::shooting::healths(&sent));
        nearest = nearest.min(feet.lock().unwrap()[2]);
    }
    assert!(raised.is_some_and(|tick| tick < 5), "raised at once, under 10 off: {raised:?}");
    assert!(!events.is_empty() && swings == 0, "{events:?}, {swings} swings");
    assert!(healths.iter().any(|health| (health - 16.5).abs() < 1e-3), "3.5 a hit on easy: {healths:?}");
    assert!(throws.iter().any(|[x, y, z]| x.abs() < 1e-3 && (y - 0.4).abs() < 1e-3 && (z + 0.4).abs() < 1e-3), "{throws:?}");
    assert!(nearest < -1.0, "it went through its prey: {nearest}");
}
