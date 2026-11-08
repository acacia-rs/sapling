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
