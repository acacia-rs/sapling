//! What armed kinds are born with and let go of at death (creatures/spawn_gear.rs): a piglin's
//! unit, armour pools by difficulty, the 1 in 4 a player's kill drops.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddItemEntity, MobArmorEquipment, MobEquipment};
use acacia_proto::types::ItemV4Extra;
use common::{Harness, all};
use sapling_api::{Attacker, DamageCause, Difficulty, EntityId, GameMode, PlayerId};

const ALEX: PlayerId = PlayerId(1);

fn harness(difficulty: Difficulty) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| {
        ctx.set_game_mode(ALEX, GameMode::Spectator);
        ctx.set_difficulty(difficulty);
    });
    h.tick_all();
    h
}

fn ticks(h: &mut Harness, n: usize) -> Vec<RawPacket> {
    (0..n).flat_map(|_| h.tick_all().to(1).to_vec()).collect()
}

/// `count` of `kind` summoned by `event`, and what Alex was sent over their first 5 ticks.
fn summoned(h: &mut Harness, kind: &'static str, count: usize, event: Option<&'static str>) -> (Vec<EntityId>, Vec<RawPacket>) {
    let made = Arc::new(Mutex::new(Vec::new()));
    let out = made.clone();
    h.remote.run(move |ctx| *out.lock().unwrap() = (0..count).map(|at| ctx.summon_as(None, kind, [2.5 + (at % 8) as f32, 65.0, 2.5 + (at / 8) as f32], 0.0, 0.0, event).expect("summoned")).collect());
    let sent = ticks(h, 5);
    let made = std::mem::take(&mut *made.lock().unwrap());
    (made, sent)
}

/// The first main hand each of `mobs` was shown holding.
fn held(sent: &[RawPacket], mobs: &[EntityId]) -> BTreeMap<u64, Option<&'static str>> {
    let mut held = BTreeMap::new();
    for shown in all::<MobEquipment>(sent).into_iter().filter(|shown| shown.slot == 0 && mobs.iter().any(|mob| mob.0 == shown.runtime_entity_id)) {
        held.entry(shown.runtime_entity_id).or_insert_with(|| sapling_data::item_name(shown.item.network_id.into()));
    }
    held
}

#[test]
fn an_adult_piglin_comes_holding_a_crossbow_or_a_golden_sword_by_its_unit_and_a_baby_holds_nothing() {
    let mut h = harness(Difficulty::Easy);
    let (piglins, sent) = summoned(&mut h, "minecraft:piglin", 24, None);
    let held = held(&sent, &piglins);
    assert_eq!(held.len(), piglins.len(), "each shown holding: {held:?}");
    let crossbows = held.values().filter(|item| **item == Some("minecraft:crossbow")).count();
    let melee = held.values().filter(|item| matches!(item, Some("minecraft:golden_sword" | "minecraft:golden_spear"))).count();
    assert!(crossbows > 0 && melee > 0 && crossbows + melee == piglins.len(), "{held:?}");
    let (ranged, sent) = summoned(&mut h, "minecraft:piglin", 4, Some("spawn_adult_ranged"));
    assert!(self::held(&sent, &ranged).values().all(|item| *item == Some("minecraft:crossbow")), "spawn_adult_ranged");
    let (babies, sent) = summoned(&mut h, "minecraft:piglin", 4, Some("minecraft:entity_born"));
    assert!(self::held(&sent, &babies).values().all(Option::is_none), "babies hold nothing");
    let (unitless, sent) = summoned(&mut h, "minecraft:piglin", 4, Some("minecraft:spawn_adult"));
    assert!(self::held(&sent, &unitless).values().all(Option::is_none), "an event the pack lacks gives no unit (bds-goldpickup.txt)");
}

/// How many of `mobs` were shown wearing a piece.
fn armoured(sent: &[RawPacket], mobs: &[EntityId]) -> usize {
    let worn = all::<MobArmorEquipment>(sent).into_iter().filter(|worn| [&worn.helmet, &worn.chestplate, &worn.leggings, &worn.boots].iter().any(|piece| piece.network_id != 0));
    worn.map(|worn| worn.runtime_entity_id).filter(|id| mobs.iter().any(|mob| mob.0 == *id)).collect::<BTreeSet<_>>().len()
}

/// skeleton_gear.json's armour pool: `random_difficulty_chance` 0 on easy, and on hard
/// `random_regional_difficulty_chance` 0.15 of the special multiplier, 1 there.
#[test]
fn skeletons_on_hard_come_in_armour_about_one_in_seven_and_never_on_easy() {
    let mut h = harness(Difficulty::Hard);
    let (skeletons, sent) = summoned(&mut h, "minecraft:skeleton", 64, None);
    let worn = armoured(&sent, &skeletons);
    assert!((2..=24).contains(&worn), "{worn} of 64 in armour");
    let mut h = harness(Difficulty::Easy);
    let (skeletons, sent) = summoned(&mut h, "minecraft:skeleton", 64, None);
    assert_eq!(armoured(&sent, &skeletons), 0);
}

fn is_sword(left: &AddItemEntity) -> bool {
    matches!(sapling_data::item_name(left.item.network_id.into()), Some("minecraft:golden_sword" | "minecraft:golden_spear"))
}

/// The golden swords and spears left lying, each with whether its tag holds a `Damage`.
fn swords_left(sent: &[RawPacket]) -> Vec<bool> {
    all::<AddItemEntity>(sent).into_iter().filter(is_sword).map(|left| matches!(&left.item.extra, ItemV4Extra::Default(Some(extra)) if format!("{:?}", extra.nbt).contains("Damage"))).collect()
}

/// `Mob::dropEquipmentOnDeath` (1429f04f0): a slot with no `slot_drop_chance` drops 1 time in 4,
/// only when a player slew the mob, worn short of new; in the death's tick, 0.05 above the loot
/// that follows a tick later (bds-ad3drops.txt).
#[test]
fn zombie_pigmen_a_player_slays_drop_their_sword_one_time_in_four_worn_and_none_die_alone() {
    let mut h = harness(Difficulty::Normal);
    let (pigmen, _) = summoned(&mut h, "minecraft:zombie_pigman", 48, None);
    ticks(&mut h, 40);
    h.remote.run(move |ctx| {
        let alex = Attacker::Player(ctx.players().find(|player| player.id == ALEX).expect("Alex"));
        pigmen.iter().for_each(|pigman| assert!(ctx.hurt_entity_by(*pigman, 200.0, DamageCause::Attack, &alex)));
    });
    let (first, rest) = (ticks(&mut h, 1), ticks(&mut h, 4));
    assert_eq!(swords_left(&rest), Vec::<bool>::new(), "all in the death's tick");
    let left = swords_left(&first);
    assert!((4..=24).contains(&left.len()), "{} of 48 left their sword", left.len());
    assert!(left.iter().all(|worn| *worn), "each worn: {left:?}");
    let heights = |sent: &[RawPacket], swords: bool| all::<AddItemEntity>(sent).into_iter().filter(|left| is_sword(left) == swords).map(|left| left.position.y).collect::<Vec<_>>();
    let (gear, loot) = (heights(&first, true), heights(&rest, false));
    assert!(loot.iter().any(|y| gear.iter().any(|gear| (gear - y - 0.05).abs() < 1e-4)), "gear {gear:?} 0.05 over loot {loot:?}");
    let (pigmen, _) = summoned(&mut h, "minecraft:zombie_pigman", 48, None);
    h.remote.run(move |ctx| pigmen.iter().for_each(|pigman| assert!(ctx.kill_entity(*pigman))));
    assert_eq!(swords_left(&ticks(&mut h, 5)), Vec::<bool>::new(), "none without a player's kill");
}
