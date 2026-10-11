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
