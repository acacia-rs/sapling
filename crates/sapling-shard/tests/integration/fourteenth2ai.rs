//! Group AI of the fourteenth wave: how far a baby looks for a grown one of its kind (FollowParentGoal's
//! box, read under gdb in bds-ai2kin), and mobs pushed up a vine (bds-ai2push).

use crate::common;

use common::asking::ask;
use sapling_api::{Block, Difficulty, EntityId, GameMode, PlayerId};

const FLOOR: f32 = 65.0;

fn feet(h: &mut common::Harness, id: u64) -> [f32; 3] {
    ask(h, move |ctx| ctx.entity(EntityId(id)).expect("still there").feet)
}

/// bds-ai2kin.txt: babies 9, 9.75 and 11 off along x took no grown one until a stroll brought them
/// within 8.68 (the baby's box grown 8 meeting the grown one's); one 7 off walked to it at once.
#[test]
fn a_piglet_nine_off_does_not_take_up_a_grown_pig_and_one_seven_off_does() {
    let mut h = common::Harness::new();
    h.enter(1, "Alex");
    let (far, near) = ask(&mut h, |ctx| {
        ctx.set_game_mode(PlayerId(1), GameMode::Creative);
        ctx.teleport(PlayerId(1), [0.5, FLOOR, -7.5]);
        ctx.summon_as(None, "minecraft:pig", [-2.5, FLOOR, 4.5], 0.0, 0.0, Some("minecraft:spawn_adult")).expect("a pig");
        let far = ctx.summon_as(None, "minecraft:pig", [6.5, FLOOR, 4.5], 270.0, 0.0, Some("minecraft:entity_born")).expect("a piglet").0;
        ctx.summon_as(None, "minecraft:pig", [-2.5, FLOOR, -3.5], 0.0, 0.0, Some("minecraft:spawn_adult")).expect("a pig");
        let near = ctx.summon_as(None, "minecraft:pig", [4.5, FLOOR, -3.5], 270.0, 0.0, Some("minecraft:entity_born")).expect("a piglet").0;
        (far, near)
    });
    let (far_from, near_from) = (feet(&mut h, far), feet(&mut h, near));
    for _ in 0..20 {
        ask(&mut h, |_| ());
    }
    let (far_to, near_to) = (feet(&mut h, far), feet(&mut h, near));
    assert!(near_from[0] - near_to[0] > 1.5, "the piglet 7 off stayed: {near_from:?} {near_to:?}");
    assert!((far_from[0] - far_to[0]).abs() < 0.3, "the piglet 9 off set off: {far_from:?} {far_to:?}");
}

/// bds-ai2push.txt: zombies crowded into a vine cell walled on three sides were pushed against its
/// walls and went up it 0.2 a tick to its top.
#[test]
fn zombies_crowded_into_a_walled_vine_cell_climb_it() {
    let mut h = common::Harness::new();
    h.enter(1, "Alex");
    let zombies = ask(&mut h, |ctx| {
        ctx.set_game_mode(PlayerId(1), GameMode::Creative);
        ctx.set_difficulty(Difficulty::Easy);
        ctx.teleport(PlayerId(1), [-6.5, FLOOR, -6.5]);
        for y in 65..=70 {
            for [x, z] in [[1, -1], [1, 0], [1, 1], [2, -1], [2, 1], [3, -1], [3, 0], [3, 1]] {
                assert!(ctx.set_block([x, y, z], &Block::new("minecraft:stone")));
            }
            assert!(ctx.set_block([2, y, 0], &Block::new("minecraft:vine")));
        }
        ["2.3", "2.5", "2.7"].map(|x| ctx.summon_as(None, "minecraft:zombie", [x.parse().unwrap(), FLOOR, 0.45], 90.0, 0.0, None).expect("a zombie").0)
    });
    let mut heights = Vec::new();
    for _ in 0..40 {
        let ids = zombies;
        heights.push(ask(&mut h, move |ctx| ids.map(|id| ctx.entity(EntityId(id)).map_or(f32::NAN, |seen| seen.feet[1]))));
    }
    let top = heights.iter().flatten().copied().fold(FLOOR, f32::max);
    assert!(top > FLOOR + 3.0, "no zombie climbed: {heights:?}");
    let rises: Vec<f32> = heights.windows(2).flat_map(|pair| (0..3).map(move |i| pair[1][i] - pair[0][i])).filter(|rise| *rise > 0.01).collect();
    assert!(rises.iter().all(|rise| *rise <= 0.2 + 1e-4), "climbed faster than 0.2 a tick: {rises:?}");
}
