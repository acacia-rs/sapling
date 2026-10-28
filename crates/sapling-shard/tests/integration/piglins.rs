//! Piglins' bartering, against research/captures/bds-barter.txt and the pack.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, AddItemEntity, LevelSoundEvent, MobEquipment, TakeItemEntity};
use acacia_proto::types::WindowID;
use common::raising::click_on;
use common::stored::{enter_stored, floor_with};
use common::{Harness, all, config};
use sapling_api::{GameMode, Item, PlayerId, Slot};
use sapling_world::{ChunkPos, World, WorldStorage, stored_actors};

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
        // A nugget it may take straight back up, as BDS's did (bds-goldpickup.txt).
        assert!(!all::<AddItemEntity>(&admiring[emptied]).is_empty(), "its barter drops by it");
    }
}

/// A piglin by Alex in creative, and what Alex was told of it; its id.
fn piglin_by_alex(h: &mut Harness) -> u64 {
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        assert!(ctx.spawn_entity("minecraft:piglin", [3.5, 65.0, 0.5]).is_some());
    });
    let shown: Vec<RawPacket> = (0..2).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    all::<AddEntity>(&shown).into_iter().find(|added| added.entity_type == "minecraft:piglin").expect("shown").runtime_id
}

/// Drops `item` at `at` and ticks until the piglin takes it: the ticks taken and all that was told.
fn dropped_for(h: &mut Harness, piglin: u64, item: &'static str, at: [f32; 3]) -> (usize, Vec<Vec<RawPacket>>) {
    h.remote.run(move |ctx| assert!(ctx.drop_item(&Item::new(item, 1), at)));
    let ticks: Vec<Vec<RawPacket>> = (0..80).map(|_| h.tick_all().to(1).to_vec()).collect();
    let took = ticks.iter().position(|sent| all::<TakeItemEntity>(sent).iter().any(|taken| taken.target as u64 == piglin)).unwrap_or_else(|| panic!("{item} never taken"));
    (took, ticks)
}

#[test]
fn a_piglin_walks_to_gold_lying_near_admires_an_ingot_and_barters_it_and_keeps_nuggets_in_hand() {
    let mut h = Harness::new();
    let piglin = piglin_by_alex(&mut h);
    let (took, ticks) = dropped_for(&mut h, piglin, "minecraft:gold_nugget", [6.5, 65.5, 0.5]);
    let held: Vec<(WindowID, i16)> = all::<MobEquipment>(&ticks[took + 1]).into_iter().filter(|held| held.runtime_entity_id == piglin).map(|held| (held.window_id, held.item.network_id)).collect();
    assert!(held.len() == 1 && held[0].0 == WindowID::Inventory && held[0].1 != 0, "a nugget in its main hand a tick on: {held:?}");

    let (took, ticks) = dropped_for(&mut h, piglin, "minecraft:gold_ingot", [6.5, 65.5, -2.5]);
    assert!(offhand(&ticks[took + 1], piglin).first().is_some_and(|item| *item != 0), "the ingot in its offhand a tick on");
    assert!(all::<LevelSoundEvent>(&ticks[took + 2]).iter().any(|sound| sound.sound_id == "admire"), "and admire heard a tick after");

    let mut shown: Vec<RawPacket> = h.enter(2, "Steve").to(2).to_vec();
    shown.extend((0..3).flat_map(|_| h.tick_all().to(2).to_vec()));
    assert!(all::<MobEquipment>(&shown).iter().any(|held| held.runtime_entity_id == piglin && held.window_id == WindowID::Inventory && held.item.network_id != 0), "one who comes later sees the nugget");
}

#[test]
fn a_piglin_admires_a_golden_sword_it_picks_up_and_keeps_it_but_leaves_cobblestone() {
    let mut h = Harness::new();
    let piglin = piglin_by_alex(&mut h);
    h.remote.run(|ctx| assert!(ctx.drop_item(&Item::new("minecraft:cobblestone", 1), [3.5, 65.5, 2.0])));
    let (took, ticks) = dropped_for(&mut h, piglin, "minecraft:golden_sword", [6.5, 65.5, 0.5]);
    assert!(ticks[..took].iter().all(|sent| all::<TakeItemEntity>(sent).is_empty()), "the cobblestone is left");
    let after: Vec<Vec<RawPacket>> = (0..170).map(|_| h.tick_all().to(1).to_vec()).collect();
    let emptied = after.iter().find(|sent| offhand(sent, piglin) == [0]).expect("its hand empties");
    let hands: Vec<WindowID> = all::<MobEquipment>(emptied).into_iter().filter(|held| held.runtime_entity_id == piglin).map(|held| held.window_id).collect();
    assert_eq!(hands, [WindowID::Inventory, WindowID::Offhand], "the sword to its main hand as the offhand empties (bds-piglinkeep.txt)");
    assert!(after.iter().all(|sent| all::<AddItemEntity>(sent).is_empty()), "and nothing is handed back");
}

/// bds-piglinkeep.txt, bds-piglinkeep2.txt: the sword it kept and the ingot it was admiring come
/// back with the world, shown in its hands.
#[test]
fn what_a_piglin_holds_is_saved_and_shown_again() {
    let storage = floor_with(Vec::new());
    let mut h = Harness::with(config(), World::with_storage(storage.clone()));
    enter_stored(&mut h, 1);
    h.remote.run(|ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        assert!(ctx.spawn_entity("minecraft:piglin", [8.5, 64.0, 8.5]).is_some());
    });
    let shown: Vec<RawPacket> = (0..2).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let piglin = all::<AddEntity>(&shown).into_iter().find(|added| added.entity_type == "minecraft:piglin").expect("shown").runtime_id;
    dropped_for(&mut h, piglin, "minecraft:golden_sword", [11.5, 64.5, 8.5]);
    (0..170).for_each(|_| drop(h.tick_all()));
    dropped_for(&mut h, piglin, "minecraft:gold_ingot", [11.5, 64.5, 10.5]);
    h.shard.save_worlds();
    let column = storage.load_column(ChunkPos::new(0, 0), 0).unwrap().expect("the column");
    let saved = stored_actors(&column.actors).unwrap().into_iter().find_map(|(actor, ..)| actor.filter(|actor| actor.identifier == "minecraft:piglin")).expect("saved");
    let name = |held: &Option<sapling_world::ItemStack>| held.as_ref().map(|held| held.name.clone());
    assert_eq!((name(&saved.mainhand), name(&saved.offhand)), (Some("minecraft:golden_sword".into()), Some("minecraft:gold_ingot".into())));

    let mut again = Harness::with(config(), World::with_storage(storage));
    let sent = enter_stored(&mut again, 1);
    let piglin = all::<AddEntity>(&sent).into_iter().find(|added| added.entity_type == "minecraft:piglin").expect("shown again").runtime_id;
    let hands: Vec<WindowID> = all::<MobEquipment>(&sent).into_iter().filter(|held| held.runtime_entity_id == piglin && held.item.network_id != 0).map(|held| held.window_id).collect();
    assert_eq!(hands, [WindowID::Inventory, WindowID::Offhand]);
}
