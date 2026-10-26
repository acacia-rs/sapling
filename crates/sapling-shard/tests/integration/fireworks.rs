//! Firework rockets launched off a block and from a crossbow (docs/research/wither-lightning-fireworks.md).

use crate::common;

use acacia_proto::packets::{EntityEvent, EntityEventEventId, RemoveEntity};
use common::shooting::{self, ALEX, added, draw, held, throw, watch};
use common::{Harness, all, click};
use sapling_api::{GameMode, Item, Slot};

const ROCKET: &str = "minecraft:firework_rocket";
const FLYING: &str = "minecraft:fireworks_rocket";

#[test]
fn a_rocket_off_a_block_rises_bursts_within_its_life_and_is_spent() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new(ROCKET, 2)))));
    h.tick_all();
    h.receive(1, &click([2, 64, 0], 1, None));
    let sent = watch(&mut h, 30);
    let rocket = added(&sent, FLYING).into_iter().next().expect("launched");
    assert!((rocket.position.x, rocket.position.y, rocket.position.z) == (2.5, 65.0, 0.5), "{:?}", rocket.position);
    assert!((rocket.velocity.y - 0.03).abs() < 0.002, "{:?}", rocket.velocity);
    let burst = all::<EntityEvent>(&sent).into_iter().any(|event| event.runtime_entity_id == rocket.runtime_id && event.event_id == EntityEventEventId::FireworkParticles);
    let gone = all::<RemoveEntity>(&sent).iter().any(|gone| gone.entity_id_self == rocket.unique_id);
    assert!(burst && gone, "a plain rocket lives 10 to 22 ticks");
    assert_eq!(held(&mut h, Slot::Main(0)).map(|item| item.count), Some(1));
}

#[test]
fn a_crossbow_loads_a_rocket_from_the_offhand_and_shoots_it_along_the_look() {
    let mut h = shooting::ready(GameMode::Survival, &[("minecraft:crossbow", 1, 0)], 0.0);
    h.remote.run(|ctx| assert!(ctx.set_item(ALEX, Slot::Offhand, Some(&Item::new(ROCKET, 1)))));
    h.tick_all();
    draw(&mut h, 26, 1);
    assert!(held(&mut h, Slot::Offhand).is_none(), "the rocket is loaded");
    let sent = throw(&mut h, 2);
    let rocket = added(&sent, FLYING).into_iter().next().expect("shot");
    // BDS's starts slow along the look, as one off a block does upwards.
    assert!((rocket.velocity.z - 0.03).abs() < 0.002 && rocket.velocity.y.abs() < 0.002, "{:?}", rocket.velocity);
    let moved = all::<acacia_proto::packets::MoveEntityDelta>(&watch(&mut h, 5)).len();
    assert!(moved > 0, "it flies on");
}
