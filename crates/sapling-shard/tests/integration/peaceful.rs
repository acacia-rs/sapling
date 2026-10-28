//! Peaceful takes the monsters away (src/spawning/peaceful.rs).

use crate::common;

use acacia_proto::packets::{AddItemEntity, RemoveEntity};
use common::shooting::spawn;
use common::{Harness, all};
use sapling_api::Difficulty;

#[test]
fn peaceful_removes_monsters_without_drops_but_keeps_piglins_and_animals() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| ctx.set_difficulty(Difficulty::Easy));
    let zombie = spawn(&mut h, "minecraft:zombie", [3.5, 65.0, 3.5]);
    let brute = spawn(&mut h, "minecraft:piglin_brute", [-3.5, 65.0, 3.5]);
    let piglin = spawn(&mut h, "minecraft:piglin", [3.5, 65.0, -3.5]);
    let cow = spawn(&mut h, "minecraft:cow", [-3.5, 65.0, -3.5]);
    let before: Vec<i64> = (0..5).flat_map(|_| all::<RemoveEntity>(h.tick_all().to(1))).map(|gone| gone.entity_id_self).collect();
    assert!(before.is_empty(), "{before:?}");
    h.remote.run(|ctx| ctx.set_difficulty(Difficulty::Peaceful));
    let sent: Vec<_> = (0..3).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let mut gone: Vec<i64> = all::<RemoveEntity>(&sent).iter().map(|gone| gone.entity_id_self).collect();
    gone.sort();
    let mut expected = vec![zombie as i64, brute as i64];
    expected.sort();
    assert_eq!(gone, expected, "piglin {piglin} and cow {cow} stay");
    assert!(all::<AddItemEntity>(&sent).is_empty(), "no drops");
}
