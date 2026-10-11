//! Armour wears by what it stops (docs/research/player-damage.md, "Armour wear").

mod common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{InventorySlot, PlayerArmorDamage};
use common::{Harness, all};
use sapling_api::{DamageCause, Item, PlayerId, Slot};

const ALEX: PlayerId = PlayerId(1);

/// What the hit told its player each piece has lost, if it told any, and the slots it sent.
fn hit(h: &mut Harness, amount: f32, cause: DamageCause) -> (Vec<Vec<i16>>, usize) {
    (0..12).for_each(|_| drop(h.tick_all()));
    h.remote.run(move |ctx| assert!(ctx.hurt_player(ALEX, amount, cause)));
    let sent = h.tick_all();
    let own: Vec<RawPacket> = [1, 2].iter().flat_map(|conn| sent.to(*conn).to_vec()).collect();
    (all::<PlayerArmorDamage>(&own).iter().map(|told| told.entries.iter().map(|entry| entry.damage).collect()).collect(), all::<InventorySlot>(&own).len())
}

#[test]
fn armour_wears_by_a_quarter_of_a_hit_it_stops_and_a_helmet_by_what_falls_on_it() {
    // research/captures/bds-wear.txt: all of iron.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.remote.run(|ctx| {
        for (slot, piece) in ["helmet", "chestplate", "leggings", "boots"].into_iter().enumerate() {
            assert!(ctx.set_item(ALEX, Slot::Armor(slot as u8), Some(&Item::new(&format!("minecraft:iron_{piece}"), 1))));
        }
    });
    assert_eq!(hit(&mut h, 2.0, DamageCause::Magic), (vec![], 0), "magic wears nothing");
    assert_eq!(hit(&mut h, 2.0, DamageCause::Contact), (vec![vec![1; 4]], 0));
    assert_eq!(hit(&mut h, 8.0, DamageCause::Contact), (vec![vec![3; 4]], 0), "8 wears by two");
    // The anvil's 2 wears each by one and then the helmet, sent anew, by 8 to 11 more.
    assert_eq!(hit(&mut h, 2.0, DamageCause::Anvil), (vec![vec![4; 4]], 1));
    let (told, slots) = hit(&mut h, 2.0, DamageCause::Fire);
    assert!(slots == 0 && told.len() == 1 && told[0][1..] == [5; 3] && (13..=16).contains(&told[0][0]), "{told:?}");
}

#[test]
fn slowness_slows_a_player_by_the_modifier_bds_wrote() {
    // research/captures/bds-fx.txt.
    use acacia_proto::packets::UpdateAttributes;
    use sapling_api::Effect;
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.remote.run(|ctx| assert!(ctx.give_effect(ALEX, Effect::Slowness, 400, 3, true)));
    let sent = h.tick_all();
    let own: Vec<RawPacket> = [1, 2].iter().flat_map(|conn| sent.to(*conn).to_vec()).collect();
    let paces: Vec<(f32, String, f32)> = all::<UpdateAttributes>(&own).iter().flat_map(|told| told.attributes.clone()).filter(|told| told.name == "minecraft:movement").map(|told| (told.current, told.modifiers[0].name.clone(), told.modifiers[0].amount)).collect();
    assert!(paces.iter().all(|(pace, name, amount)| (pace - 0.04).abs() < 1e-6 && name == "MovementSlowdown" && (amount + 0.6).abs() < 1e-6) && !paces.is_empty(), "{paces:?}");
    assert_eq!((Effect::ALL.len(), Effect::ALL.iter().filter(|effect| effect.name() == "levitation").count()), (25, 1));
    // Health boost: 4 more a level at most, told with its modifier (bds-fx.txt).
    h.remote.run(|ctx| assert!(ctx.give_effect(ALEX, Effect::HealthBoost, 400, 1, true)));
    let sent = h.tick_all();
    let own: Vec<RawPacket> = [1, 2].iter().flat_map(|conn| sent.to(*conn).to_vec()).collect();
    let most: Vec<(f32, f32, String)> = all::<UpdateAttributes>(&own).iter().flat_map(|told| told.attributes.clone()).filter(|told| told.name == "minecraft:health" && !told.modifiers.is_empty()).map(|told| (told.current, told.max, told.modifiers[0].name.clone())).collect();
    assert!(!most.is_empty() && most.iter().all(|told| *told == (20.0, 28.0, "HealthBoost".to_owned())), "{most:?}");
}
