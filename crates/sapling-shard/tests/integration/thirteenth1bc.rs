//! Group BC's `/event` residue (research/captures/bds-bc1event.txt): what an event changes is told after the
//! command's answer, sheep wool, colour and growing events change the sheep, and a player's
//! `minecraft:gain_raid_omen` swaps its bad omen for the raid omen.

use crate::common;

use acacia_proto::Packet;
use acacia_proto::packets::{AddItemEntity, MobEffect, MobEffectEventId, SetEntityData, Text};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1 as Flags};
use common::raising::{has_flag, set_in};
use common::shooting::{ALEX, ready, spawn, watch};
use common::{all, asking::ask};
use sapling_api::{Effect, EntityId, GameMode};

/// What Alex is told in the tick `event` is fired on the mob `id` and the one after.
fn fire(h: &mut common::Harness, id: u64, event: &'static str) -> Vec<acacia_proto::RawPacket> {
    h.remote.run(move |ctx| assert!(ctx.fire_entity_event(EntityId(id), event), "{event}"));
    [h.tick_all().to(1).to_vec(), watch(h, 1)].concat()
}

#[test]
fn an_events_changes_are_told_after_the_answer_sent_while_it_ran() {
    let mut h = ready(GameMode::Creative, &[], 0.0);
    let sheep = spawn(&mut h, "minecraft:sheep", [3.5, 65.0, 0.5]);
    h.remote.run(move |ctx| {
        assert!(ctx.fire_entity_event(EntityId(sheep), "wololo"));
        ctx.send_message(ALEX, "answer");
    });
    let sent = h.tick_all().to(1).to_vec();
    let first = |id: u32| sent.iter().position(|packet| packet.id == id);
    assert!(first(Text::ID) < first(SetEntityData::ID) && first(SetEntityData::ID).is_some(), "answer, then the colour");
    assert_eq!(set_in(&sent, sheep, Key::Color), Some(Plain::Byte(14)), "wololo turns it red");
}

#[test]
fn sheep_events_shear_regrow_recolour_and_grow_it() {
    let mut h = ready(GameMode::Creative, &[], 0.0);
    let sheep = spawn(&mut h, "minecraft:sheep", [3.5, 65.0, 0.5]);
    assert_eq!(has_flag(&fire(&mut h, sheep, "minecraft:on_sheared"), sheep, Flags::SHEARED), Some(true));
    assert_eq!(has_flag(&fire(&mut h, sheep, "minecraft:on_eat_block"), sheep, Flags::SHEARED), Some(false));
    let coloured = fire(&mut h, sheep, "minecraft:cold_color");
    assert!(matches!(set_in(&coloured, sheep, Key::Color), Some(Plain::Byte(15 | 8 | 7 | 0 | 12 | 6))), "one of cold_color's six");
    assert!(ask(&mut h, move |ctx| ctx.fire_entity_event(EntityId(sheep), "minecraft:entity_born")));
    watch(&mut h, 2);
    let grown = fire(&mut h, sheep, "minecraft:ageable_grow_up");
    assert_eq!(set_in(&grown, sheep, Key::Scale), Some(Plain::Float(1.0)));
    assert_eq!(set_in(&grown, sheep, Key::BoundingboxWidth), None, "its box is unchanged");
    assert!(all::<AddItemEntity>(&watch(&mut h, 5)).is_empty(), "an evented grow-up drops nothing");
}

#[test]
fn a_players_gain_raid_omen_swaps_its_bad_omen_for_a_30_s_raid_omen() {
    let mut h = ready(GameMode::Survival, &[], 0.0);
    h.remote.run(|ctx| assert!(ctx.give_effect(ALEX, Effect::BadOmen, 2000, 2, false)));
    watch(&mut h, 2);
    h.remote.run(|ctx| assert!(ctx.fire_player_event(ALEX, "minecraft:gain_raid_omen") && ctx.fire_player_event(ALEX, "minecraft:trigger_raid")));
    let effects: Vec<(MobEffectEventId, i32, i32, i32)> = all::<MobEffect>(h.tick_all().to(1)).into_iter().map(|effect| (effect.event_id, effect.effect_id, effect.amplifier, effect.duration)).collect();
    assert!(effects.iter().any(|&(event, id, ..)| event == MobEffectEventId::Remove && id == 28), "{effects:?}");
    assert!(effects.contains(&(MobEffectEventId::Add, 36, 0, 600)), "{effects:?}");
    assert!(!ask(&mut h, |ctx| ctx.fire_player_event(ALEX, "gain_raid_omen")), "the pack's name only");
}
