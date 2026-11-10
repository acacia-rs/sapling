//! Group AO of the fourteenth wave: a mob on a small raised floor strolls about it, toward places it
//! cannot reach (bds-am2push).

use crate::common;

use common::asking::ask;
use sapling_api::{Block, EntityId, GameMode, PlayerId};

const TOP: f32 = 71.0;

/// bds-am2push.txt: zombies on a 3×3 stone top six up walked about it every few seconds, 0.3 to 1.3
/// at a time and never off it: a stroll's place is moved down its column to the floor below, and
/// the way goes to the cell of the top nearest it.
#[test]
fn a_mob_on_a_small_raised_floor_strolls_about_it() {
    let mut h = common::Harness::with(sapling_shard::ShardConfig { mobs_stroll: true, ..common::config() }, common::platform());
    h.enter(1, "Alex");
    let cow = ask(&mut h, |ctx| {
        ctx.set_game_mode(PlayerId(1), GameMode::Spectator);
        for y in 65..TOP as i32 {
            for x in 1..=3 {
                for z in -1..=1 {
                    assert!(ctx.set_block([x, y, z], &Block::new("minecraft:stone")));
                }
            }
        }
        ctx.spawn_entity("minecraft:cow", [2.5, TOP, 0.5]).expect("a cow").0
    });
    let mut feet = Vec::new();
    for _ in 0..1800 {
        feet.push(ask(&mut h, move |ctx| ctx.entity(EntityId(cow)).expect("still there").feet));
    }
    let on_top = feet.iter().all(|at| at[1] == TOP && (1.0..4.0).contains(&at[0]) && (-1.0..2.0).contains(&at[2]));
    let moving: Vec<bool> = feet.windows(2).map(|pair| pair[0][0] != pair[1][0] || pair[0][2] != pair[1][2]).collect();
    let walks = moving.windows(2).filter(|pair| !pair[0] && pair[1]).count();
    assert!(on_top, "left the top: {:?}", feet.iter().find(|at| !(at[1] == TOP && (1.0..4.0).contains(&at[0]))));
    // A tick in 120 sets off (15 in 1800), some toward places whose nearest cell is where it stands.
    assert!(walks >= 5, "{walks} walks");
}
