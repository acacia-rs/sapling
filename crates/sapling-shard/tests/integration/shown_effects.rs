//! A player's effects as it and those who see it are shown them: `VisibleMobEffects`, as BDS's
//! (research/captures/bds-effects.txt).

use crate::common;

use acacia_proto::{Packet, RawPacket};
use acacia_proto::packets::SetEntityData;
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use common::{Harness, all};
use sapling_api::{Effect, PlayerId};

const ALEX: PlayerId = PlayerId(1);

/// The `VisibleMobEffects` of Alex in `sent`.
fn shown(sent: &[RawPacket]) -> Vec<i64> {
    let items = all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == 1).flat_map(|data| data.metadata);
    items.filter(|item| item.key == Key::VisibleMobEffects).filter_map(|item| if let Value::Default(Plain::Long(packed)) = item.value { Some(packed) } else { None }).collect()
}

#[test]
fn a_players_effects_with_particles_show_to_it_and_its_viewers() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.remote.run(|ctx| assert!(ctx.give_effect(ALEX, Effect::Regeneration, 40, 0, true) && ctx.give_effect(ALEX, Effect::Poison, 40, 0, true) && ctx.give_effect(ALEX, Effect::Speed, 40, 0, false)));
    let sent = h.tick_all();
    assert_eq!((shown(sent.to(1)), shown(sent.to(2))), (vec![20, 20 << 7 | 38], vec![20, 20 << 7 | 38]), "the lower id higher; speed hides its particles");
    h.remote.run(|ctx| assert!(ctx.remove_effect(ALEX, Effect::Speed) && ctx.remove_effect(ALEX, Effect::Regeneration)));
    assert_eq!(shown(h.tick_all().to(2)), [38], "no change for the hidden one");
    let gone: Vec<RawPacket> = (0..42).flat_map(|_| h.tick_all().to(2).to_vec()).collect();
    assert_eq!(shown(&gone), [0]);
}

#[test]
fn effects_stand_still_while_dead_and_are_told_gone_by_id_at_the_rise() {
    use acacia_proto::packets::{MobEffect, MobEffectEventId, Respawn};
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| assert!(ctx.give_effect(ALEX, Effect::Regeneration, 1200, 0, true) && ctx.give_effect(ALEX, Effect::Speed, 1200, 1, true)));
    (0..10).for_each(|_| drop(h.tick_all()));
    h.remote.run(|ctx| assert!(ctx.kill_player(ALEX)));
    (0..30).for_each(|_| drop(h.tick_all()));
    h.receive(1, &Respawn { position: acacia_proto::types::Vec3f { x: 0.0, y: 0.0, z: 0.0 }, state: 2, runtime_entity_id: 0 });
    let risen = h.tick_all().to(1).to_vec();
    let gone: Vec<(i32, i32)> = all::<MobEffect>(&risen).iter().filter(|effect| effect.event_id == MobEffectEventId::Remove).map(|effect| (effect.effect_id, effect.duration)).collect();
    // bds-r1fx.txt: speed 1120 and regeneration 1160 left after 80 and 40 ticks alive, two seconds dead.
    assert!(matches!(gone.as_slice(), [(1, a), (10, b)] if a == b && (1185..=1191).contains(a)), "{gone:?}");
}

/// Gives or clears Alex's effects as a command would, before the tick's effects are counted: by
/// what Alex says.
struct Commanding;

impl sapling_api::Handler for Commanding {
    fn on_chat(&mut self, ctx: &mut sapling_api::ShardCtx<'_>, _: &sapling_api::Player, event: &mut sapling_api::ChatEvent) {
        match event.message.as_str() {
            "heal" => assert!(ctx.give_effect(ALEX, Effect::InstantHealth, 1, 0, true)),
            "speed" => assert!(ctx.give_effect(ALEX, Effect::Speed, 600, 0, true)),
            _ => assert!(ctx.clear_effects(ALEX)),
        }
    }
}

fn says(message: &str) -> acacia_proto::packets::Text {
    use acacia_proto::packets::{Text, TextCategory, TextContent, TextContentChat, TextType};
    let content = TextContent::Chat(TextContentChat { source_name: "Alex".into(), message: message.into() });
    Text { needs_translation: false, category: TextCategory::Authored, r#type: TextType::Chat, content, xuid: String::new(), platform_chat_id: String::new(), has_filtered_message: false, filtered_message: None }
}

#[test]
fn an_effect_given_by_a_command_is_first_counted_the_tick_after() {
    use acacia_proto::packets::{MobEffect, MobEffectEventId};
    let mut h = Harness::new();
    h.shard.add_handler_at(sapling_api::Priority::Normal, Box::new(Commanding));
    h.enter(1, "Alex");
    let told = |sent: &[RawPacket]| all::<MobEffect>(sent).iter().map(|effect| (effect.event_id.clone(), effect.duration)).collect::<Vec<_>>();
    let ticks = |sent: &[RawPacket]| all::<MobEffect>(sent).iter().map(|effect| effect.tick).collect::<Vec<_>>();
    // bds-r1fx.txt: instant health told gone a tick after its Add; speed 30 cleared a second in told 581.
    h.receive(1, &says("heal"));
    let added = h.tick_all().to(1).to_vec();
    assert_eq!(told(&added), [(MobEffectEventId::Add, 1)]);
    let removed = h.tick_all().to(1).to_vec();
    assert_eq!(told(&removed), [(MobEffectEventId::Remove, 0)]);
    // Each is stamped with the tick it is told in, as BDS's were (bds-r1buckets.txt).
    let [stamp] = ticks(&added)[..] else { panic!("left: one Add") };
    assert_eq!(ticks(&removed), [stamp + 1], "left: Remove stamp");
    assert!(stamp > 0, "left: stamp {stamp}");
    h.receive(1, &says("speed"));
    (0..20).for_each(|_| drop(h.tick_all()));
    h.receive(1, &says("clear"));
    assert_eq!(told(h.tick_all().to(1)), [(MobEffectEventId::Remove, 581)]);
}

#[test]
fn a_viewer_coming_later_finds_a_mobs_effects_in_its_announcement() {
    // research/captures/bds-lateeffects.txt: the rejoined actor's AddEntity of a cow under speed carried 2.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let id = common::asking::ask(&mut h, |ctx| ctx.spawn_entity("minecraft:cow", [3.5, 64.0, 0.5]).expect("a cow"));
    // Slowness with its particles hidden is not shown.
    h.remote.run(move |ctx| assert!(ctx.give_entity_effect(id, Effect::Speed, 600, 1) && ctx.give_entity_effect_shown(id, Effect::Slowness, 600, 255, false)));
    h.tick_all();
    let mut sent: Vec<RawPacket> = h.enter(2, "Steve").to(2).to_vec();
    (0..5).for_each(|_| sent.extend(h.tick_all().to(2).to_vec()));
    let cow = all::<acacia_proto::packets::AddEntity>(&sent).into_iter().find(|added| added.entity_type == "minecraft:cow").expect("the cow announced");
    let visible = cow.metadata.iter().find(|item| item.key == Key::VisibleMobEffects).map(|item| item.value.clone());
    assert_eq!(visible, Some(Value::Default(Plain::Long(2))));
}

#[test]
fn a_viewer_coming_later_finds_a_players_effects_in_its_announcement() {
    // research/captures/bds-lateeffects.txt: the rejoined actor's AddPlayer of the watcher under jump boost carried 16.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| assert!(ctx.give_effect(ALEX, Effect::JumpBoost, 1200, 0, true)));
    h.tick_all();
    let mut sent: Vec<RawPacket> = h.enter(2, "Steve").to(2).to_vec();
    (0..5).for_each(|_| sent.extend(h.tick_all().to(2).to_vec()));
    let alex = all::<acacia_proto::packets::AddPlayer>(&sent).into_iter().find(|added| added.username == "Alex").expect("Alex announced");
    let visible = alex.metadata.iter().find(|item| item.key == Key::VisibleMobEffects).map(|item| item.value.clone());
    assert_eq!(visible, Some(Value::Default(Plain::Long(16))));
    assert!(all::<acacia_proto::packets::MobEffect>(&sent).is_empty(), "BDS told no MobEffect with it");
}

#[test]
fn a_mobs_effect_is_told_before_what_it_shows_and_a_hidden_one_shows_nothing() {
    // research/captures/bds-lateeffects.txt: speed's MobEffect then VisibleMobEffects 2; slowness
    // without particles its MobEffect alone.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let id = common::asking::ask(&mut h, |ctx| ctx.spawn_entity("minecraft:cow", [3.5, 64.0, 0.5]).expect("a cow"));
    h.tick_all();
    let shows = |p: &RawPacket| p.decode::<SetEntityData>().is_ok_and(|data| data.metadata.iter().any(|item| item.key == Key::VisibleMobEffects));
    let told = |sent: &[RawPacket]| sent.iter().filter(|p| p.id == acacia_proto::packets::MobEffect::ID || (p.id == SetEntityData::ID && shows(p))).map(|p| p.id).collect::<Vec<u32>>();
    h.remote.run(move |ctx| assert!(ctx.give_entity_effect(id, Effect::Speed, 600, 1)));
    let sent = h.tick_all();
    let shows = told(sent.to(1));
    assert_eq!(shows.first(), Some(&acacia_proto::packets::MobEffect::ID), "{shows:?}");
    assert!(all::<SetEntityData>(sent.to(1)).iter().flat_map(|data| &data.metadata).any(|item| item.key == Key::VisibleMobEffects));
    h.remote.run(move |ctx| assert!(ctx.give_entity_effect_shown(id, Effect::Slowness, 600, 255, false)));
    let sent = h.tick_all();
    assert!(!all::<SetEntityData>(sent.to(1)).iter().flat_map(|data| &data.metadata).any(|item| item.key == Key::VisibleMobEffects), "nothing more shown");
    assert_eq!(all::<acacia_proto::packets::MobEffect>(sent.to(1)).len(), 1);
}
