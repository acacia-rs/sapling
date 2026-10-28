//! What plugins show one player: hotbar lines, labels, hidden entities, floating text, extra boss
//! bars and debug shapes, as packets (docs/research/ui-events.md).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{
    AddEntity, AddPlayer, BossEvent, BossEventType, MoveEntity, RemoveEntity, ServerScriptDebugDrawer, ServerScriptDebugDrawerShapesItemShapeType as ShapeType, SetEntityData, Text, TextType,
};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use common::asking::ask;
use common::{Harness, all};
use sapling_api::{BossBar, DebugShape, EntityId, PlayerId, ShapeKind, ShardCtx};

fn after(h: &mut Harness, action: impl FnOnce(&mut ShardCtx<'_>) + Send + 'static) -> Vec<RawPacket> {
    h.remote.run(action);
    h.tick(1)
}

fn text_of(data: &[acacia_proto::types::MetadataDictionaryItem], key: Key) -> Option<String> {
    data.iter().find(|item| item.key == key).and_then(|item| if let Value::Default(Plain::String(text)) = &item.value { Some(text.clone()) } else { None })
}

/// Every string a SetEntityData among `sent` gives `key`, with the entity it is for.
fn labels(sent: &[RawPacket], key: Key) -> Vec<(u64, String)> {
    all::<SetEntityData>(sent).iter().filter_map(|data| Some((data.runtime_entity_id, text_of(&data.metadata, key)?))).collect()
}

#[test]
fn popups_tips_and_jukebox_popups_are_text_of_their_own_types() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let sent = after(&mut h, |ctx| {
        ctx.send_popup(PlayerId(1), "pop");
        ctx.send_tip(PlayerId(1), "tip");
        ctx.send_jukebox_popup(PlayerId(1), "disc");
    });
    let types: Vec<TextType> = all::<Text>(&sent).into_iter().map(|text| text.r#type).collect();
    assert_eq!(types, [TextType::Popup, TextType::Tip, TextType::JukeboxPopup]);
}

/// Alex (1) and Steve (2) at the spawn, each seeing the other; Steve's runtime id.
fn two() -> (Harness, u64) {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let sent = h.enter(2, "Steve");
    let mut shown: Vec<AddPlayer> = all(sent.to(1));
    (0..3).for_each(|_| shown.extend(all::<AddPlayer>(h.tick_all().to(1))));
    let steve = shown.iter().find(|added| added.username == "Steve").expect("Alex sees Steve").runtime_id;
    (h, steve)
}

#[test]
fn a_name_tag_and_a_score_tag_reach_who_sees_the_player_and_come_with_it_again() {
    let (mut h, steve) = two();
    let sent = after(&mut h, |ctx| {
        assert!(ctx.set_name_tag(PlayerId(2), Some("§aSteve the Great")));
        assert!(ctx.set_score_tag(PlayerId(2), Some("12 kills")));
    });
    assert_eq!(labels(&sent, Key::Nametag), [(steve, "§aSteve the Great".to_owned())]);
    assert_eq!(labels(&sent, Key::ScoreTag), [(steve, "12 kills".to_owned())]);

    // Hidden, Steve is removed from Alex's screen alone; shown again, he comes back with his labels.
    let sent = after(&mut h, |ctx| assert!(ctx.hide_from(PlayerId(1), PlayerId(2))));
    assert_eq!(all::<RemoveEntity>(&sent).iter().map(|r| r.entity_id_self as u64).collect::<Vec<_>>(), [steve]);
    assert!(all::<AddPlayer>(&h.tick(1)).is_empty(), "he stays hidden");
    let sent = after(&mut h, |ctx| assert!(ctx.show_to(PlayerId(1), PlayerId(2))));
    assert_eq!(all::<AddPlayer>(&sent).len(), 1);
    assert_eq!(labels(&sent, Key::Nametag), [(steve, "§aSteve the Great".to_owned())]);

    let sent = after(&mut h, |ctx| assert!(ctx.set_name_tag(PlayerId(2), None)));
    assert_eq!(labels(&sent, Key::Nametag), [(steve, "Steve".to_owned())], "his own name back");
}

#[test]
fn floating_text_is_a_still_nameplate_that_can_change_and_go() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let (tx, rx) = std::sync::mpsc::channel();
    h.remote.run(move |ctx| tx.send(ctx.add_floating_text([2.5, 66.0, 2.5], "Welcome").expect("the spawn is loaded")).unwrap());
    let sent: Vec<RawPacket> = (0..2).flat_map(|_| h.tick(1)).collect();
    let id = rx.recv().unwrap();
    let added = all::<AddEntity>(&sent).into_iter().find(|added| added.runtime_id == id.0).expect("shown to Alex");
    assert_eq!((added.entity_type.as_str(), text_of(&added.metadata, Key::Nametag).as_deref()), ("minecraft:falling_block", Some("Welcome")));
    let sent = after(&mut h, move |ctx| assert!(ctx.set_floating_text(id, "Bye")));
    assert_eq!(labels(&sent, Key::Nametag), [(id.0, "Bye".to_owned())]);
    let still = ask(&mut h, move |ctx| ctx.set_floating_text(EntityId(id.0 + 1000), "x"));
    assert!(!still);
    h.remote.run(move |ctx| assert!(ctx.remove_entity(id)));
    let sent: Vec<RawPacket> = (0..2).flat_map(|_| h.tick(1)).collect();
    assert_eq!(all::<RemoveEntity>(&sent).iter().map(|r| r.entity_id_self as u64).collect::<Vec<_>>(), [id.0]);
}

#[test]
fn each_extra_boss_bar_hangs_on_an_unseen_entity_of_its_own_that_follows() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let sent = after(&mut h, |ctx| {
        ctx.set_boss_bar(PlayerId(1), &BossBar::new("own"));
        ctx.set_extra_boss_bar(PlayerId(1), 7, &BossBar::new("seven"));
        ctx.set_extra_boss_bar(PlayerId(1), 3, &BossBar::new("three"));
    });
    let holders: Vec<u64> = all::<AddEntity>(&sent).iter().map(|added| added.runtime_id).collect();
    let shown: Vec<(i64, String)> = all::<BossEvent>(&sent).into_iter().filter(|e| e.r#type == BossEventType::ShowBar).map(|e| (e.target_entity_id, e.title)).collect();
    assert_eq!(holders.len(), 2);
    assert_eq!(shown[1..], [(holders[0] as i64, "seven".to_owned()), (holders[1] as i64, "three".to_owned())]);
    assert!(!holders.contains(&(shown[0].0 as u64)), "the player's own bar is on its own entity");

    let sent = after(&mut h, |ctx| ctx.set_extra_boss_bar(PlayerId(1), 7, &BossBar::new("seven").progress(0.5)));
    assert_eq!(all::<BossEvent>(&sent).iter().map(|e| (e.target_entity_id, e.r#type)).collect::<Vec<_>>(), [(holders[0] as i64, BossEventType::SetBarProgress)]);
    assert!(all::<AddEntity>(&sent).is_empty());

    h.remote.run(|ctx| ctx.teleport(PlayerId(1), [40.5, 65.0, 0.5]));
    let moved: Vec<u64> = (0..3).flat_map(|_| all::<MoveEntity>(&h.tick(1))).map(|m| m.runtime_entity_id).collect();
    assert_eq!(moved, [holders[1], holders[0]], "both follow the player, by key");

    let sent = after(&mut h, |ctx| ctx.remove_extra_boss_bar(PlayerId(1), 3));
    assert_eq!(all::<BossEvent>(&sent).iter().map(|e| (e.target_entity_id, e.r#type)).collect::<Vec<_>>(), [(holders[1] as i64, BossEventType::HideBar)]);
    assert_eq!(all::<RemoveEntity>(&sent).iter().map(|r| r.entity_id_self as u64).collect::<Vec<_>>(), [holders[1]]);
}

#[test]
fn a_debug_shape_is_drawn_and_erased_under_its_id() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let sent = after(&mut h, |ctx| {
        ctx.draw_shape(PlayerId(1), 9, &DebugShape::new([0.0, 70.0, 0.0], ShapeKind::Box { bounds: [1.0, 2.0, 3.0] }).colour([0xff, 0xff, 0, 0]));
        ctx.erase_shape(PlayerId(1), 9);
    });
    let drawn = all::<ServerScriptDebugDrawer>(&sent);
    let [drawn, erased] = [&drawn[0].shapes[0], &drawn[1].shapes[0]];
    assert_eq!((drawn.network_id, drawn.shape_type.clone(), drawn.color), (9, Some(ShapeType::Box), Some(0xffff_0000_u32 as i32)));
    assert_eq!(drawn.box_bound.as_ref().map(|b| [b.x, b.y, b.z]), Some([1.0, 2.0, 3.0]));
    assert_eq!((erased.network_id, erased.shape_type.clone(), erased.location.is_none()), (9, None, true));
}
