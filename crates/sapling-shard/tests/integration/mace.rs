//! A mace's smash, against research/captures/bds-mace.txt and bds-mace-push.txt.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, EntityEvent, EntityEventEventId, LevelEvent, LevelEventEvent, LevelSoundEvent, SetEntityMotion, UpdateAttributes};
use common::{Harness, all, attack, input_at};
use sapling_api::{GameMode, Item, PlayerId, Slot};

const ALEX: PlayerId = PlayerId(1);

fn health_of(sent: &[RawPacket], id: u64) -> Vec<f32> {
    let updates = all::<UpdateAttributes>(sent).into_iter().filter(|update| update.runtime_entity_id == id);
    updates.flat_map(|update| update.attributes).filter(|attribute| attribute.name == "minecraft:health").map(|attribute| attribute.current).collect()
}

fn spawned(h: &mut Harness, kind: &'static str, feet: [f32; 3]) -> u64 {
    h.remote.run(move |ctx| assert!(ctx.spawn_entity(kind, feet).is_some()));
    (0..3).flat_map(|_| all::<AddEntity>(h.tick_all().to(2))).collect::<Vec<_>>().remove(0).runtime_id
}

/// Alex, holding `weapon`, falls from 68.6 to `at` and hits a golem standing at 2, 65, 0.5, a cow
/// beside it; what Steve was sent, the golem and the cow.
fn smash(weapon: Item, at: f32) -> (Vec<RawPacket>, u64, u64) {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.remote.run(|ctx| ctx.set_game_mode(PlayerId(2), GameMode::Creative));
    let golem = spawned(&mut h, "minecraft:iron_golem", [2.0, 65.0, 0.5]);
    let cow = spawned(&mut h, "minecraft:cow", [4.2, 65.0, 0.5]);
    h.remote.run(move |ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&weapon))));
    for y in [66.6, 68.6, 68.5, at] {
        h.receive(1, &input_at([0.5, y, 0.5]));
        h.tick_all();
    }
    h.receive(1, &attack(golem));
    (h.tick_all().to(2).to_vec(), golem, cow)
}

#[test]
fn a_smash_adds_by_the_fall_flashes_mace_smash_and_shows_dust_and_its_sound() {
    let (sent, golem, _) = smash(Item::new("minecraft:mace", 1), 65.6);
    // A fall of 3 costs (6 + 12) x 1.5 as a critical.
    assert_eq!(health_of(&sent, golem), [73.0]);
    let flashed: Vec<i32> = all::<EntityEvent>(&sent).into_iter().filter(|event| event.runtime_entity_id == golem && event.event_id == EntityEventEventId::HurtAnimation).map(|event| event.data).collect();
    assert_eq!(flashed, [34]);
    assert!(all::<LevelEvent>(&sent).iter().any(|event| event.event == LevelEventEvent::AddParticleSmashAttackGroundDust));
    assert!(all::<LevelSoundEvent>(&sent).iter().any(|sound| sound.sound_id == "mace.smash_ground"));
}

#[test]
fn a_smash_stops_the_hitter_and_shoves_a_mob_near() {
    let (sent, _, cow) = smash(Item::new("minecraft:mace", 1), 65.6);
    let shoved = all::<SetEntityMotion>(&sent).into_iter().find(|told| told.runtime_entity_id == cow).expect("the cow was shoved");
    let stopped = all::<SetEntityMotion>(&sent).into_iter().any(|told| told.runtime_entity_id == 1 && told.velocity.y == 0.0);
    assert!(stopped, "the hitter's fall is stopped");
    // 2.2 away: 0.6 x 0.3 out, after a tick's ground drag; 0.6 up, after gravity and drag.
    assert!((shoved.velocity.x - 0.18 * 0.546).abs() < 1e-4 && (shoved.velocity.y - 0.43276805).abs() < 1e-5, "{shoved:?}");
}

#[test]
fn a_fall_short_of_one_and_a_half_is_a_plain_critical() {
    let (sent, golem, _) = smash(Item::new("minecraft:mace", 1), 67.4);
    assert_eq!(health_of(&sent, golem), [91.0]);
}
