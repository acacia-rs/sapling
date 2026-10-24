use acacia_proto::nbt::{List, Value};

use super::*;

/// A stack's NBT with these enchantments, as (enchantment, level).
fn enchanted(levels: &[(Enchant, i16)]) -> Value {
    let entry = |(enchant, level): &(Enchant, i16)| Value::Compound(vec![("id".into(), Value::Short(*enchant as i16)), ("lvl".into(), Value::Short(*level))]);
    Value::Compound(vec![("ench".into(), Value::List(List { tag: 10, items: levels.iter().map(entry).collect() }))])
}

fn always(at: f32) -> impl FnMut() -> f32 {
    move || at
}

#[test]
fn a_level_is_read_off_the_ench_list_and_none_is_zero() {
    let sword = enchanted(&[(Enchant::Sharpness, 5), (Enchant::Looting, 3), (Enchant::Knockback, -2)]);
    assert_eq!([Enchant::Sharpness, Enchant::Looting, Enchant::Smite, Enchant::Knockback].map(|enchant| level(Some(&sword), enchant)), [5, 3, 0, 0]);
    assert_eq!(level(None, Enchant::Sharpness), 0);
}

#[test]
fn efficiency_and_unbreaking() {
    assert_eq!([0, 1, 5].map(efficiency), [0.0, 2.0, 26.0]);
    assert_eq!([0, 1, 2, 3].map(|level| wear_chance(level, false)), [100, 50, 33, 25]);
    assert_eq!([0, 1, 2, 3].map(|level| wear_chance(level, true)), [100, 80, 73, 70]);
}

#[test]
fn sharpness_adds_to_every_hit_and_smite_and_bane_to_their_own_mobs() {
    let sharp = enchanted(&[(Enchant::Sharpness, 4)]);
    let smite = enchanted(&[(Enchant::Smite, 2)]);
    let bane = enchanted(&[(Enchant::BaneOfArthropods, 3)]);
    assert_eq!([None, Some("minecraft:zombie"), Some("minecraft:cow")].map(|target| melee_bonus(Some(&sharp), target)), [5.0; 3]);
    assert_eq!([Some("minecraft:zombie"), Some("minecraft:spider"), None].map(|target| melee_bonus(Some(&smite), target)), [5.0, 0.0, 0.0]);
    assert_eq!([Some("minecraft:cave_spider"), Some("minecraft:zombie")].map(|target| melee_bonus(Some(&bane), target)), [7.0, 0.0]);
    assert_eq!(melee_bonus(None, Some("minecraft:zombie")), 0.0);
    // Floored: Sharpness I to V adds 1, 2, 3, 5, 6 (BDS `getMeleeDamageBonus`).
    let sharpness = |level: i16| melee_bonus(Some(&enchanted(&[(Enchant::Sharpness, level)])), None);
    assert_eq!([1, 2, 3, 4, 5].map(sharpness), [1.0, 2.0, 3.0, 5.0, 6.0]);
}

#[test]
fn weapon_effects() {
    assert_eq!([0, 1, 2].map(aspect_burn), [0, 80, 160]);
    assert_eq!([bane_slowness(1, &mut always(0.0)), bane_slowness(3, &mut always(0.999))], [20, 49]);
    assert_eq!(BANE_SLOWNESS, 0, "BDS's first level, not the Java game's fourth");
    assert_eq!([knockback(0, [0.4, 0.4]), knockback(2, [0.4, 0.4])], [[0.4, 0.4], [1.4, 0.4]]);
}

#[test]
fn mending_takes_two_of_wear_a_point_and_keeps_the_rest() {
    assert_eq!(mend(100, 5), (10, 0));
    assert_eq!(mend(3, 5), (3, 4));
    assert_eq!(mend(0, 5), (0, 5));
}

#[test]
fn breath_frost_and_looting() {
    assert_eq!([0, 1, 3].map(holds_breath), [0.0, 0.5, 0.75]);
    assert_eq!([0, 1, 2].map(frost_reach), [0, 3, 4]);
    assert_eq!([looted([0.0, 1.0], 0, &mut always(0.999)), looted([0.0, 1.0], 3, &mut always(0.999)), looted([0.0, 1.0], 3, &mut always(0.0))], [0, 3, 0]);
    assert!((looting_chance(0.025, 0.01, 3) - 0.055).abs() < 1e-6);
}

#[test]
fn protection_points_add_up_by_kind_to_a_cap() {
    let worn = |levels: &[(Enchant, i16)]| enchanted(levels);
    let helmet = worn(&[(Enchant::Protection, 4)]);
    let boots = worn(&[(Enchant::FeatherFalling, 4), (Enchant::Protection, 4)]);
    let fiery = worn(&[(Enchant::FireProtection, 4)]);
    let warding = Warding::of([Some(&helmet), Some(&fiery), None, Some(&boots)]);
    let spared = |harm: Harm| (warding.spared(10.0, harm) * 1000.0).round() / 1000.0;
    // Protection 8 points; with Fire 16; with Feather Falling 8 + 12, at the cap of 20.
    assert_eq!([Harm::Other, Harm::Fire, Harm::Fall, Harm::Blast].map(spared), [6.8, 3.6, 2.0, 6.8]);
    assert_eq!(Warding::default().spared(10.0, Harm::Fire), 10.0);
}

#[test]
fn fire_protection_summed_over_the_pieces_shortens_a_burn() {
    let fiery = enchanted(&[(Enchant::FireProtection, 2)]);
    let warding = Warding::of([Some(&fiery), None, None, None]);
    // 160 x 0.7 = 112; Fire Protection on four pieces, 8 levels, leaves nothing.
    assert_eq!([warding.burns_for(160), warding.burns_for(15), Warding::default().burns_for(160)], [112, 11, 160]);
    assert_eq!(Warding::of([Some(&fiery); 4]).burns_for(160), 0);
}

#[test]
fn frost_walker_counts_to_six() {
    let boots = enchanted(&[(Enchant::FrostWalker, 10), (Enchant::DepthStrider, 3)]);
    let warding = Warding::of([None, None, None, Some(&boots)]);
    assert_eq!((warding.frost_walker, warding.depth_strider, frost_reach(warding.frost_walker)), (6, 3, 8));
}

#[test]
fn thorns_hurts_back_by_chance() {
    assert_eq!([thorns(3, &mut always(0.44)), thorns(3, &mut always(0.46)), thorns(0, &mut always(0.0))], [Some(2.0), None, None]);
    assert_eq!(thorns(12, &mut always(0.0)), Some(2.0));
}
