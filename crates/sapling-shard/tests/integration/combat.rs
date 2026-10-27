//! Shields, totems, critical hits, a weapon's wear and what hurts a player where it stands, against
//! research/captures/bds-{shield,totem,crit,swing-wear,thorny,void,anvil}.txt.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, Animate, AnimateActionId, EntityEvent, EntityEventEventId, LevelEvent, LevelEventEvent, LevelSoundEvent, MobEffect, UpdateAttributes};
use acacia_proto::nbt::Value;
use acacia_proto::packets::InventoryContent;
use acacia_proto::types::{InputData, ItemV4Extra, WindowIDVarint};
use common::{Harness, all, attack, input_at};
use sapling_api::{Block, DamageCause, Difficulty, GameMode, Item, PlayerId, Slot};

const ALEX: PlayerId = PlayerId(1);

fn ready() -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.remote.run(|ctx| ctx.set_game_mode(PlayerId(2), GameMode::Creative));
    h
}

fn health_of(sent: &[RawPacket], id: u64) -> Vec<f32> {
    let updates = all::<UpdateAttributes>(sent).into_iter().filter(|update| update.runtime_entity_id == id);
    updates.flat_map(|update| update.attributes).filter(|attribute| attribute.name == "minecraft:health").map(|attribute| attribute.current).collect()
}

fn sounds(sent: &[RawPacket]) -> Vec<String> {
    all::<LevelSoundEvent>(sent).into_iter().map(|sound| sound.sound_id).collect()
}

fn hurt_by(sent: &[RawPacket], id: u64) -> Vec<i32> {
    all::<EntityEvent>(sent).into_iter().filter(|event| event.runtime_entity_id == id && event.event_id == EntityEventEventId::HurtAnimation).map(|event| event.data).collect()
}

fn hold(h: &mut Harness, slot: Slot, name: &'static str) {
    h.remote.run(move |ctx| assert!(ctx.set_item(ALEX, slot, Some(&Item::new(name, 1)))));
}

/// The wear (`Damage` tag) the last showing of `window` gave its first slot; nought for no tag.
fn shown_wear(sent: &[RawPacket], window: WindowIDVarint) -> Option<i32> {
    let content = all::<InventoryContent>(sent).into_iter().filter(|content| content.window_id == window).last()?;
    let tag = match content.input.first()?.extra.clone() {
        ItemV4Extra::Default(Some(extra)) => extra.nbt.map(|nbt| nbt.nbt.value),
        ItemV4Extra::ShieldItemID(Some(extra)) => extra.nbt.map(|nbt| nbt.nbt.value),
        _ => None,
    };
    Some(match tag.as_ref().and_then(|tag| tag.get("Damage")) {
        Some(Value::Int(damage)) => *damage,
        _ => 0,
    })
}

fn spawned(h: &mut Harness, kind: &'static str, feet: [f32; 3]) -> u64 {
    h.remote.run(move |ctx| assert!(ctx.spawn_entity(kind, feet).is_some()));
    (0..3).flat_map(|_| all::<AddEntity>(h.tick_all().to(2))).collect::<Vec<_>>().remove(0).runtime_id
}

fn cow(h: &mut Harness, feet: [f32; 3]) -> u64 {
    spawned(h, "minecraft:cow", feet)
}

#[test]
fn a_totem_in_either_hand_keeps_a_player_alive_with_its_effects_and_is_used_up_but_not_against_the_void() {
    for slot in [Slot::Offhand, Slot::Main(0)] {
        let mut h = ready();
        hold(&mut h, slot, "minecraft:totem_of_undying");
        h.tick_all();
        h.remote.run(|ctx| assert!(ctx.hurt_player(ALEX, 30.0, DamageCause::Generic)));
        let sent = h.tick_all().to(2).to_vec();
        let given: Vec<(i32, i32, i32)> = all::<MobEffect>(&sent).into_iter().map(|effect| (effect.effect_id, effect.amplifier, effect.duration)).collect();
        assert_eq!(given, [(10, 1, 900), (22, 1, 100), (12, 0, 800)], "{slot:?}");
        let consumed = all::<EntityEvent>(&sent).iter().any(|event| event.event_id == EntityEventEventId::ConsumeTotem);
        let heard = all::<LevelEvent>(&sent).iter().any(|event| event.event == LevelEventEvent::SoundTotem);
        assert!(consumed && heard && health_of(&sent, 1).contains(&1.0), "{slot:?}");
        assert_eq!(common::asking::ask(&mut h, move |ctx| ctx.item(ALEX, slot)), None, "{slot:?}: used up");
    }
    let mut h = ready();
    hold(&mut h, Slot::Offhand, "minecraft:totem_of_undying");
    h.remote.run(|ctx| assert!(ctx.hurt_player(ALEX, 30.0, DamageCause::Void)));
    let sent = h.tick_all().to(1).to_vec();
    assert!(all::<EntityEvent>(&sent).iter().any(|event| event.event_id == EntityEventEventId::DeathAnimation), "the void takes no totem");
}

/// Alex sneaking with a shield in `slot`, facing south, and a zombie `ahead` blocks off.
fn guarded(slot: Slot, ahead: f32) -> (Harness, u64) {
    let mut h = ready();
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_time(18000);
    });
    hold(&mut h, slot, "minecraft:shield");
    let mut input = input_at([0.5, 65.0, 0.5]);
    input.input_data = vec![InputData::StartSneaking];
    h.receive(1, &input);
    h.tick_all();
    let zombie = spawned(&mut h, "minecraft:zombie", [0.5, 65.0, 0.5 + ahead]);
    (h, zombie)
}

#[test]
fn a_raised_shield_turns_away_a_hit_from_in_front_wearing_by_one_more_than_the_hit_and_throws_the_hitter_back() {
    let (mut h, zombie) = guarded(Slot::Offhand, 2.0);
    let (mut sent, mut own) = (Vec::new(), Vec::new());
    for _ in 0..60 {
        let tick = h.tick_all();
        sent.extend(tick.to(2).to_vec());
        own.extend(tick.to(1).to_vec());
        if sounds(&sent).iter().any(|sound| sound == "item.shield.block") {
            break;
        }
    }
    assert!(sounds(&sent).iter().any(|sound| sound == "item.shield.block"), "blocked");
    assert!(health_of(&sent, 1).is_empty() && hurt_by(&sent, 1).is_empty(), "nothing taken");
    // 3 on normal is 2.5 on easy: the shield wears 1 + 2.
    assert_eq!(shown_wear(&own, WindowIDVarint::Offhand), Some(3));
    let thrown = all::<acacia_proto::packets::SetEntityMotion>(&sent).into_iter().filter(|thrown| thrown.runtime_entity_id == zombie).count();
    let moved = (0..3).flat_map(|_| all::<acacia_proto::packets::MoveEntityDelta>(h.tick_all().to(2))).any(|moved| moved.runtime_entity_id == zombie && moved.z.is_some_and(|z| z > 2.5));
    assert!(thrown > 0 || moved, "the zombie is thrown back");
}

#[test]
fn a_shield_does_not_turn_away_a_hit_from_behind() {
    let (mut h, _) = guarded(Slot::Offhand, -2.0);
    let mut sent = Vec::new();
    for _ in 0..60 {
        sent.extend(h.tick_all().to(2).to_vec());
        if !health_of(&sent, 1).is_empty() {
            break;
        }
    }
    assert_eq!(health_of(&sent, 1), [17.5]);
    assert!(!sounds(&sent).iter().any(|sound| sound == "item.shield.block"));
}

#[test]
fn a_sprinting_hit_throws_a_player_further_and_lower() {
    // bds-sprint.txt: 0.4 across and up walking, 0.6 across and 0.32 up sprinting.
    let mut h = ready();
    h.remote.run(|ctx| {
        ctx.set_game_mode(PlayerId(2), GameMode::Survival);
        ctx.teleport(PlayerId(2), [0.5, 65.0, 2.5]);
    });
    let mut input = input_at([0.5, 65.0, 0.5]);
    input.input_data = vec![InputData::StartSprinting];
    h.receive(1, &input);
    h.tick_all();
    h.receive(1, &attack(2));
    let thrown: Vec<[f32; 2]> = all::<acacia_proto::packets::SetEntityMotion>(h.tick_all().to(2)).into_iter().map(|thrown| [thrown.velocity.z.abs() + thrown.velocity.x.abs(), thrown.velocity.y]).collect();
    assert!(thrown.len() == 1 && (thrown[0][0] - 0.6).abs() < 1e-5 && (thrown[0][1] - 0.32).abs() < 1e-5, "{thrown:?}");
}

#[test]
fn a_hit_while_falling_is_critical_for_half_again_with_its_sound_and_sparks() {
    let mut h = ready();
    let cow = cow(&mut h, [2.0, 65.0, 0.5]);
    for y in [65.6, 66.1, 65.9] {
        h.receive(1, &input_at([0.5, y, 0.5]));
        h.tick_all();
    }
    h.receive(1, &attack(cow));
    let sent = h.tick_all().to(2).to_vec();
    assert_eq!((health_of(&sent, cow), sounds(&sent)), (vec![8.5], vec!["attack.critical".to_owned()]));
    let sparks: Vec<(u64, f32)> = all::<Animate>(&sent).into_iter().filter(|animate| animate.action_id == AnimateActionId::CriticalHit).map(|animate| (animate.runtime_entity_id, animate.data)).collect();
    assert_eq!(sparks, [(cow, 55.0)]);
}

#[test]
fn a_hit_wears_a_sword_by_one_and_an_axe_by_two_and_shows_the_hitter_its_windows() {
    let mut h = ready();
    for (name, wear, at) in [("minecraft:wooden_sword", 1, [2.0, 65.0, 0.5]), ("minecraft:wooden_axe", 2, [0.5, 65.0, 2.0]), ("minecraft:stick", 0, [-1.0, 65.0, 0.5])] {
        hold(&mut h, Slot::Main(0), name);
        let cow = cow(&mut h, at);
        h.receive(1, &attack(cow));
        let sent = h.tick_all().to(1).to_vec();
        assert_eq!(shown_wear(&sent, WindowIDVarint::Inventory), Some(wear), "{name}");
    }
}

#[test]
fn a_cactus_pricks_a_player_against_it_and_the_void_hurts_one_under_the_world() {
    let mut h = ready();
    h.remote.run(|ctx| assert!(ctx.set_block([1, 65, 0], &Block::new("cactus"))));
    h.receive(1, &input_at([0.75, 65.0, 0.5]));
    let sent: Vec<RawPacket> = (0..3).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    assert_eq!((health_of(&sent, 1), hurt_by(&sent, 1)), (vec![19.0], vec![DamageCause::Contact.id()]));
    h.remote.run(|ctx| ctx.teleport(ALEX, [0.5, -110.0, 0.5]));
    let sent: Vec<RawPacket> = (0..3).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    assert!(hurt_by(&sent, 1).contains(&DamageCause::Void.id()), "{:?}", hurt_by(&sent, 1));
}

#[test]
fn a_falling_anvil_costs_two_for_each_block_past_the_first() {
    let mut h = ready();
    h.remote.run(|ctx| assert!(ctx.update_block([0, 71, 0], &Block::new("anvil"))));
    let mut sent = Vec::new();
    for _ in 0..40 {
        h.receive(1, &input_at([0.5, 65.0, 0.5]));
        sent.extend(h.tick_all().to(1).to_vec());
    }
    assert_eq!((health_of(&sent, 1), hurt_by(&sent, 1)), (vec![10.0], vec![DamageCause::Anvil.id()]));
}
