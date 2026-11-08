//! Group BD's omens (research/captures/bds-bd2event.txt, bds-bd2mouth.txt): bad omen never shows particles,
//! and each omen is heard as it takes hold, not as it is raised again. The selector's order is pinned in
//! sapling-api's selector tests, the mob leaving an edge in body/tests.rs.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{LevelSoundEvent, MobEffect};
use common::shooting::{ALEX, ready, spawn, watch};
use common::{all, asking::ask};
use sapling_api::{Effect, EntityId, GameMode};

fn omens_heard(packets: &[RawPacket]) -> Vec<LevelSoundEvent> {
    all::<LevelSoundEvent>(packets).into_iter().filter(|sound| sound.sound_id.starts_with("apply_effect.")).collect()
}

#[test]
fn bad_omen_shows_no_particles_and_an_omen_is_heard_as_it_takes_hold() {
    let mut h = ready(GameMode::Creative, &[], 0.0);
    watch(&mut h, 2);
    h.remote.run(|ctx| assert!(ctx.give_effect(ALEX, Effect::BadOmen, 600, 0, true)));
    let sent = h.tick_all().to(1).to_vec();
    let given: Vec<(i32, bool)> = all::<MobEffect>(&sent).into_iter().map(|effect| (effect.effect_id, effect.particles)).collect();
    assert_eq!(given, [(28, false)], "asked with particles, shown without");
    let heard: Vec<(String, String)> = omens_heard(&sent).into_iter().map(|sound| (sound.sound_id, sound.entity_type)).collect();
    assert_eq!(heard, [("apply_effect.bad_omen".to_owned(), "minecraft:player".to_owned())]);
    assert!(ask(&mut h, |ctx| ctx.give_effect(ALEX, Effect::BadOmen, 600, 1, true)));
    assert!(omens_heard(&watch(&mut h, 1)).is_empty(), "raised again, silent");
    h.remote.run(|ctx| assert!(ctx.give_effect(ALEX, Effect::RaidOmen, 600, 0, true)));
    let sent = h.tick_all().to(1).to_vec();
    assert!(all::<MobEffect>(&sent).iter().any(|effect| effect.effect_id == 36 && effect.particles), "raid omen keeps its particles");
    assert_eq!(omens_heard(&sent).iter().map(|sound| sound.sound_id.as_str()).collect::<Vec<_>>(), ["apply_effect.raid_omen"]);
}

#[test]
fn a_mobs_omen_is_heard_at_its_eyes_ahead_of_it() {
    let mut h = ready(GameMode::Creative, &[], 0.0);
    let sheep = spawn(&mut h, "minecraft:sheep", [3.5, 65.0, 0.5]);
    watch(&mut h, 2);
    h.remote.run(move |ctx| assert!(ctx.give_entity_effect(EntityId(sheep), Effect::TrialOmen, 600, 0)));
    let heard = omens_heard(h.tick_all().to(1));
    let [sound] = &heard[..] else { panic!("one sound: {heard:?}") };
    assert_eq!((sound.sound_id.as_str(), sound.entity_type.as_str()), ("apply_effect.trial_omen", "minecraft:sheep"));
    assert!((sound.position.y - 66.105).abs() < 1e-3, "0.85 of 1.3 up: {sound:?}");
    let ahead = (sound.position.x - 3.5).hypot(sound.position.z - 0.5);
    assert!((ahead - 0.81).abs() < 0.05, "0.9 of 0.9 ahead (bds-bd2event.txt): {sound:?}");
}
