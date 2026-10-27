//! Piglins' bartering, against research/captures/bds-barter.txt and the pack.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, LevelSoundEvent, MobEquipment};
use acacia_proto::types::WindowID;
use common::asking::ask;
use common::raising::click_on;
use common::{Harness, all};
use sapling_api::{GameMode, Item, PlayerId, Slot};

const ALEX: PlayerId = PlayerId(1);

fn offhand(sent: &[RawPacket], id: u64) -> Vec<i16> {
    all::<MobEquipment>(sent).into_iter().filter(|held| held.runtime_entity_id == id && held.window_id == WindowID::Offhand).map(|held| held.item.network_id).collect()
}

#[test]
fn a_piglin_given_gold_admires_it_8_seconds_in_its_offhand_then_hands_back_its_barter() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:gold_ingot", 4))));
        assert!(ctx.spawn_entity("minecraft:piglin", [0.5, 65.0, 2.5]).is_some());
    });
    let shown: Vec<RawPacket> = (0..2).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let piglin = all::<AddEntity>(&shown).into_iter().find(|added| added.entity_type == "minecraft:piglin").expect("shown").runtime_id;
    let taken = click_on(&mut h, piglin);
    assert!(offhand(&taken, piglin).first().is_some_and(|item| *item != 0), "the ingot in its offhand");
    let heard = h.tick_all().to(1).to_vec();
    assert!(all::<LevelSoundEvent>(&heard).iter().any(|sound| sound.sound_id == "admire"));
    let admiring: Vec<Vec<RawPacket>> = (0..161).map(|_| h.tick_all().to(1).to_vec()).collect();
    let emptied = admiring.iter().position(|sent| offhand(sent, piglin) == [0]).expect("its hand empties");
    assert_eq!(emptied, 159, "161 ticks after it took the ingot");
    // An older mob-loot.json lacks the table (sapling-data README); the drop needs it.
    if sapling_data::has_loot_table("loot_tables/entities/piglin_barter.json") {
        let items = ask(&mut h, |ctx| ctx.entities().into_iter().filter(|entity| entity.item.is_some()).count());
        assert!(items > 0, "its barter lies by it");
    }
}
