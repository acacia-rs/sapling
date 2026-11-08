//! Piglins' bartering, against research/captures/bds-barter.txt and the pack.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, AddItemEntity, InventoryContent, LevelSoundEvent, MobEquipment, TakeItemEntity};
use acacia_proto::types::{ContainerSlotType as Container, ItemStackRequestActionsItemContent as Action, ItemStackRequestActionsItemContentDrop as Throw, MetadataFlags1, WindowID};
use common::raising::{click_on, has_flag};
use common::requests::{request, slot};
use common::stored::{enter_stored, floor_with};
use common::{Harness, all, config};
use sapling_api::{GameMode, Item, PlayerId, Slot};
use sapling_world::{ChunkPos, World, WorldStorage, stored_actors};

const ALEX: PlayerId = PlayerId(1);

fn offhand(sent: &[RawPacket], id: u64) -> Vec<i16> {
    all::<MobEquipment>(sent).into_iter().filter(|held| held.runtime_entity_id == id && held.window_id == WindowID::Offhand).map(|held| held.item.network_id).collect()
}

/// Bit 30 of each `FlagsExtended` the piglin was told with.
fn admiring_bit(sent: &[RawPacket], id: u64) -> Vec<bool> {
    use acacia_proto::types::MetadataDictionaryItemValue as Meta;
    let data = all::<acacia_proto::packets::SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == id);
    data.flat_map(|data| data.metadata).filter_map(|item| if let Meta::FlagsExtended(flags) = item.value { Some(flags.0 & 1 << 30 != 0) } else { None }).collect()
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
    assert_eq!(admiring_bit(&taken, piglin), [true], "shown admiring (bds-goldpickup.txt)");
    let heard = h.tick_all().to(1).to_vec();
    assert!(all::<LevelSoundEvent>(&heard).iter().any(|sound| sound.sound_id == "admire"));
    let admiring: Vec<Vec<RawPacket>> = (0..161).map(|_| h.tick_all().to(1).to_vec()).collect();
    let emptied = admiring.iter().position(|sent| offhand(sent, piglin) == [0]).expect("its hand empties");
    assert_eq!(emptied, 159, "161 ticks after it took the ingot");
    assert_eq!(admiring_bit(&admiring[emptied], piglin), [false], "no longer admiring");
    assert!(all::<LevelSoundEvent>(&admiring[emptied]).iter().any(|sound| sound.sound_id == "admire"), "admire again as it empties, Alex holding gold (bds-gxbarter.txt)");
    // An older mob-loot.json lacks the table (sapling-data README); the drop needs it.
    if sapling_data::has_loot_table("loot_tables/entities/piglin_barter.json") {
        // bds-t2barter.txt: a tick on, from a metre up, 0.3 up and 0.2 of the way to Alex's eyes.
        assert!(all::<AddItemEntity>(&admiring[emptied]).is_empty());
        let handed = all::<AddItemEntity>(&admiring[emptied + 1]);
        let (at, thrown) = (handed[0].position.clone(), handed[0].velocity.clone());
        let to = [0.5 - at.x, 65.0 + sapling_data::EYE_HEIGHT - (at.y - 1.125), 0.5 - at.z];
        let far = to.iter().map(|c| c * c).sum::<f32>().sqrt();
        assert!((at.y - 66.125).abs() < 1e-4 && (thrown.x - to[0] / far * 0.2).abs() < 1e-4 && (thrown.y - 0.3).abs() < 1e-6 && (thrown.z - to[2] / far * 0.2).abs() < 1e-4, "{handed:?}");
    }
}

/// bds-gxbarter.txt: a piglin hit 2 s before refused an ingot (`cooldown_after_being_attacked` 20).
#[test]
fn a_piglin_hit_refuses_gold_handed_to_it() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:gold_ingot", 4))));
        assert!(ctx.spawn_entity("minecraft:piglin", [0.5, 65.0, 2.5]).is_some());
    });
    let shown: Vec<RawPacket> = (0..2).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let piglin = all::<AddEntity>(&shown).into_iter().find(|added| added.entity_type == "minecraft:piglin").expect("shown").runtime_id;
    h.receive(1, &common::attack_from(piglin, [0.5, 65.0, 0.5]));
    let after: Vec<RawPacket> = (0..40).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    assert!(offhand(&after, piglin).is_empty());
    let offered = click_on(&mut h, piglin);
    assert!(offhand(&offered, piglin).iter().all(|item| *item == 0), "refused");
}

/// A piglin holding nothing by Alex in creative (bds-ah3nugget.txt: one holding a weapon keeps nuggets out of
/// sight); its id.
fn piglin_by_alex(h: &mut Harness) -> u64 {
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        assert!(ctx.summon_as(None, "minecraft:piglin", [3.5, 65.0, 0.5], 0.0, 0.0, Some("minecraft:spawn_adult")).is_some());
    });
    let shown: Vec<RawPacket> = (0..2).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    all::<AddEntity>(&shown).into_iter().find(|added| added.entity_type == "minecraft:piglin").expect("shown").runtime_id
}

/// Drops `item` at `at` and ticks until the piglin takes it: the ticks taken and all that was told.
fn dropped_for(h: &mut Harness, piglin: u64, item: &'static str, at: [f32; 3]) -> (usize, Vec<Vec<RawPacket>>) {
    h.remote.run(move |ctx| assert!(ctx.drop_item(&Item::new(item, 1), at)));
    // Stops 3 ticks after it is taken: out of the Nether the piglin zombifies 303 ticks in.
    let mut ticks: Vec<Vec<RawPacket>> = Vec::new();
    let taken = |sent: &Vec<RawPacket>| all::<TakeItemEntity>(sent).iter().any(|taken| taken.target as u64 == piglin);
    while ticks.len() < 80 && ticks.iter().position(taken).is_none_or(|took| ticks.len() < took + 3) {
        ticks.push(h.tick_all().to(1).to_vec());
    }
    let took = ticks.iter().position(taken).unwrap_or_else(|| panic!("{item} never taken"));
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

/// bds-ah3nugget.txt: the crossbow piglin took 4 nuggets with its hands shown unchanged and left
/// the 4 when it died.
#[test]
fn a_piglin_holding_a_sword_keeps_nuggets_unseen_and_drops_them_at_death() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let spawned = std::sync::Arc::new(std::sync::Mutex::new(None));
    let found = spawned.clone();
    h.remote.run(move |ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        *found.lock().unwrap() = ctx.spawn_entity("minecraft:piglin", [3.5, 65.0, 0.5]);
    });
    let shown: Vec<RawPacket> = (0..2).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let piglin = all::<AddEntity>(&shown).into_iter().find(|added| added.entity_type == "minecraft:piglin").expect("shown").runtime_id;
    let id = spawned.lock().unwrap().expect("spawned");
    dropped_for(&mut h, piglin, "minecraft:golden_sword", [6.5, 65.5, 0.5]);
    (0..170).for_each(|_| drop(h.tick_all()));
    h.remote.run(|ctx| assert!(ctx.drop_item(&Item::new("minecraft:gold_nugget", 3), [6.5, 65.5, -1.5])));
    // Killed before 303 ticks, when out of the Nether it turns zombified.
    let ticks: Vec<RawPacket> = (0..40).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    assert!(all::<TakeItemEntity>(&ticks).iter().any(|taken| taken.target as u64 == piglin), "the nuggets taken");
    assert!(all::<MobEquipment>(&ticks).iter().all(|held| held.runtime_entity_id != piglin), "its sword still shown");
    h.remote.run(move |ctx| assert!(ctx.kill_entity(id)));
    let left: Vec<RawPacket> = (0..3).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let nuggets: Vec<u16> =
        all::<AddItemEntity>(&left).into_iter().filter(|item| sapling_data::item_name(item.item.network_id.into()) == Some("minecraft:gold_nugget")).map(|item| item.item.count).collect();
    assert_eq!(nuggets, [3]);
    // Its sword, not dropped by a death no player dealt, is shown again in its main hand.
    let hands: Vec<(u8, Option<&str>)> = all::<MobEquipment>(&left).iter().filter(|held| held.runtime_entity_id == piglin).map(|held| (held.slot, sapling_data::item_name(held.item.network_id.into()))).collect();
    assert_eq!(hands, [(0, Some("minecraft:golden_sword")), (1, None)]);
}

/// bds-ah3nugget.txt: nuggets the actor threw lay 9 ticks before the piglin took them.
#[test]
fn a_piglin_takes_thrown_nuggets_9_ticks_after_they_are_shown() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        assert!(ctx.spawn_entity("minecraft:piglin", [0.5, 65.0, 3.5]).is_some());
        _ = ctx.give(ALEX, &Item::new("minecraft:gold_nugget", 4));
    });
    let sent: Vec<RawPacket> = (0..3).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let window = all::<InventoryContent>(&sent).into_iter().filter(|window| window.input.len() == 36).last().expect("the inventory").input;
    let at = window.iter().position(|item| sapling_data::item_name(item.network_id.into()) == Some("minecraft:gold_nugget")).expect("the nuggets");
    let source = slot(Container::Hotbar, at as u8, window[at].stack_id.expect("a stack id"));
    h.receive(1, &request(-3, vec![Action::Drop(Throw { count: 4, source, randomly: false })]));
    let (mut shown, mut taken) = (None, None);
    for tick in 0..40 {
        let sent = h.tick_all().to(1).to_vec();
        shown = shown.or((!all::<AddItemEntity>(&sent).is_empty()).then_some(tick));
        taken = taken.or((!all::<TakeItemEntity>(&sent).is_empty()).then_some(tick));
    }
    let (Some(shown), Some(taken)) = (shown, taken) else { panic!("shown {shown:?}, taken {taken:?}") };
    assert_eq!(taken - shown, 9);
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

/// bds-t2barter.txt: out of the Nether a piglin shook a tick after it came and was a zombified
/// piglin 303 ticks after, with its sword, heard `converted_to_zombified` from its middle.
#[test]
fn a_piglin_out_of_the_nether_shakes_and_turns_zombified_303_ticks_on_keeping_its_sword() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        // On peaceful the zombified piglin would be taken away as it came.
        ctx.set_difficulty(sapling_api::Difficulty::Easy);
        assert!(ctx.spawn_entity("minecraft:piglin", [3.5, 65.0, 0.5]).is_some());
    });
    let mut ticks: Vec<Vec<RawPacket>> = (0..2).map(|_| h.tick_all().to(1).to_vec()).collect();
    let came = ticks.iter().position(|sent| all::<AddEntity>(sent).iter().any(|added| added.entity_type == "minecraft:piglin")).expect("shown");
    let piglin = all::<AddEntity>(&ticks[came]).into_iter().find(|added| added.entity_type == "minecraft:piglin").unwrap().runtime_id;
    h.remote.run(move |ctx| assert!(ctx.set_mob_item(sapling_api::EntityId(piglin), sapling_api::MobSlot::MainHand, Some(&Item::new("minecraft:golden_sword", 1))).is_ok()));
    ticks.extend((0..320).map(|_| h.tick_all().to(1).to_vec()));
    let shook = ticks.iter().position(|sent| has_flag(sent, piglin, MetadataFlags1::VIBRATING) == Some(true)).expect("it shakes");
    assert_eq!(shook, came + 1, "a tick after it came");
    let turned = ticks.iter().position(|sent| all::<AddEntity>(sent).iter().any(|added| added.entity_type == "minecraft:zombie_pigman")).expect("it turns");
    assert_eq!(turned, came + 303);
    let sent = &ticks[turned];
    let new = all::<AddEntity>(sent).into_iter().find(|added| added.entity_type == "minecraft:zombie_pigman").unwrap().runtime_id;
    let sounds: Vec<String> = all::<LevelSoundEvent>(sent).into_iter().map(|sound| sound.sound_id).collect();
    assert!(sounds.contains(&"converted_to_zombified".to_owned()) && !sounds.contains(&"spawn".to_owned()), "{sounds:?}");
    assert!(all::<MobEquipment>(sent).iter().any(|held| held.runtime_entity_id == new && held.window_id == WindowID::Inventory && held.item.network_id != 0), "the sword kept");
}
