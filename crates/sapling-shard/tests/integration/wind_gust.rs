//! A player's wind charge flies at 2.25 a tick and its burst pushes mobs near it (bds-q3wind.txt).

use crate::common;

use acacia_proto::packets::SetEntityMotion;
use common::all;
use common::shooting::{added, ready, spawn, throw};
use sapling_api::{Block, GameMode};

#[test]
fn a_wind_charge_flies_at_2_25_and_its_burst_pushes_a_cow_aside() {
    let mut h = ready(GameMode::Creative, &[("minecraft:wind_charge", 4, 0)], 0.0);
    h.remote.run(|ctx| {
        for (x, y) in (-3..=3).flat_map(|x| (65..=67).map(move |y| (x, y))) {
            assert!(ctx.set_block([x, y, 6], &Block::new("stone")));
        }
    });
    h.tick_all();
    let cow = spawn(&mut h, "minecraft:cow", [-1.0, 65.0, 5.3]);
    let sent = throw(&mut h, 6);
    let charge = added(&sent, "minecraft:wind_charge_projectile");
    let speed = charge.first().map(|added| (added.velocity.x.powi(2) + added.velocity.y.powi(2) + added.velocity.z.powi(2)).sqrt());
    assert!(speed.is_some_and(|speed| (speed - 2.25).abs() < 0.05), "speed {speed:?}");
    let pushed = all::<SetEntityMotion>(&sent).into_iter().find(|motion| motion.runtime_entity_id == cow).map(|motion| motion.velocity);
    assert!(pushed.as_ref().is_some_and(|push| push.x < -0.1 && push.z < 0.0), "pushed {pushed:?}");
}
