//! Villagers: taking a job site, the trading screen and trades as a vanilla client makes them,
//! levelling up, restocking at work and what the world folder keeps (docs/research/villagers.md,
//! research/captures/bds-trade2.txt).

use crate::common;

use acacia_proto::nbt::Value;
use acacia_proto::packets::{AddEntity, ContainerClose, SetEntityData, UpdateTrade};
use acacia_proto::types::{
    ContainerSlotType as Container, ItemStackRequestActionsItemContent as Action, ItemStackRequestActionsItemContentCraftRecipe as Craft, ItemStackResponsesItemStatus as Status,
    MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Meta, MetadataDictionaryItemValueDefault as Plain, TransactionTransactionData,
    TransactionTransactionDataItemUseOnEntityActionType as OnEntity, WindowID, WindowType,
};
use common::requests::{answer, consume, place, request, slot, take};
use common::stored::{enter_stored, floor_holding};
use common::{Harness, all, attack, config};
use sapling_world::{ChunkPos, ItemStack, PlayerData, World, WorldStorage};

const COAL: &str = "minecraft:coal";
const FURNACE: &str = "minecraft:blast_furnace";

/// Steve with two stacks of coal, the floor holding `blocks`, and a villager summoned at `feet`
/// by `event`; its runtime id.
fn villager(blocks: &[([i32; 3], &str)], feet: [f32; 3], event: &'static str) -> (Harness, std::sync::Arc<sapling_world::MemoryStorage>, u64) {
    let storage = floor_holding(Vec::new(), blocks);
    let coal = Some(ItemStack { name: COAL.into(), count: 64, damage: 0, block: None, tag: None });
    storage.store_player(&acacia_auth::login::offline_identity("Steve"), &PlayerData { main: vec![coal.clone(), coal], ..PlayerData::default() }).unwrap();
    let mut h = Harness::with(config(), World::with_storage(storage.clone()));
    enter_stored(&mut h, 1);
    h.remote.run(move |ctx| assert!(ctx.summon_as(None, "minecraft:villager_v2", feet, 0.0, 0.0, Some(event)).is_some()));
    let sent: Vec<_> = (0..3).flat_map(|_| h.tick(1)).collect();
    let id = all::<AddEntity>(&sent).iter().find(|added| added.entity_type == "minecraft:villager_v2").expect("the villager shown").runtime_id;
    (h, storage, id)
}

fn metadata(sent: &[acacia_proto::RawPacket], id: u64, key: Key) -> Vec<i32> {
    let data = all::<SetEntityData>(sent);
    let items = data.iter().filter(|data| data.runtime_entity_id == id).flat_map(|data| data.metadata.iter().filter(|item| item.key == key));
    items.filter_map(|item| if let Meta::Default(Plain::Int(value)) = item.value { Some(value) } else { None }).collect()
}

fn use_on(id: u64) -> acacia_proto::packets::InventoryTransaction {
    let mut used = attack(id);
    if let TransactionTransactionData::ItemUseOnEntity(on) = &mut used.transaction.transaction_data {
        on.action_type = OnEntity::Interact;
    }
    used
}

/// Opens the villager's screen: its window and offers.
fn open(h: &mut Harness, id: u64) -> (u8, Vec<Value>) {
    h.receive(1, &use_on(id));
    let sent = h.tick(1);
    let screens = all::<UpdateTrade>(&sent);
    assert_eq!(screens.len(), 1, "one screen");
    let screen = &screens[0];
    assert_eq!((screen.window_type, screen.display_name.as_str(), screen.new_trading_ui, screen.economic_trades), (WindowType::Trading, "entity.villager.armor", true, true));
    let offers = screen.offers.decode().unwrap().value;
    let Some(Value::List(recipes)) = offers.get("Recipes") else { panic!("no recipes") };
    (screen.window_id.to_raw() as u8, recipes.items.clone())
}

fn int(offer: &Value, key: &str) -> i32 {
    if let Some(Value::Int(value)) = offer.get(key) { *value } else { panic!("{key}") }
}

/// One trade of the offer `net_id` for 15 coal from the payment slot, its emerald to the cursor.
fn trade(h: &mut Harness, net_id: i32) -> Vec<acacia_proto::RawPacket> {
    let actions = vec![
        Action::CraftRecipe(Craft { recipe_network_id: net_id as u32, times_crafted: 1 }),
        consume(15, slot(Container::Trade2Ingredient1, 4, 0)),
        take(1, slot(Container::CreativeOutput, 50, 0), slot(Container::Cursor, 0, 0)),
    ];
    h.receive(1, &request(-3, actions));
    let sent = h.tick(1);
    assert_eq!(answer(&sent).0, Status::Ok);
    sent
}

fn pay(h: &mut Harness, from: u8, count: u8) {
    h.receive(1, &request(-1, vec![place(count, slot(Container::Hotbar, from, 0), slot(Container::Trade2Ingredient1, 4, 0))]));
    assert_eq!(answer(&h.tick(1)).0, Status::Ok);
}

fn close(h: &mut Harness, window: u8) -> Vec<acacia_proto::RawPacket> {
    h.receive(1, &ContainerClose { window_id: WindowID::from_raw(window.into()), window_type: WindowType::None, server: false });
    h.tick(1)
}

#[test]
fn an_unemployed_villager_takes_the_job_site_beside_it() {
    let (mut h, _, id) = villager(&[([6, 64, 3], FURNACE)], [3.5, 64.0, 3.5], "minecraft:become_unskilled");
    let mut professions = Vec::new();
    for _ in 0..150 {
        professions.extend(metadata(&h.tick(1), id, Key::Variant));
    }
    assert_eq!(professions, [8], "an armorer");
    h.remote.run(|ctx| assert!(ctx.set_block([6, 64, 3], &sapling_api::Block::new("air"))));
    let after: Vec<i32> = (0..300).flat_map(|_| metadata(&h.tick(1), id, Key::Variant)).collect();
    assert_eq!(after, [] as [i32; 0], "it keeps its profession without its site (bds-n1drop.txt)");
}

#[test]
fn an_armorer_trades_coal_for_emeralds_and_reaches_apprentice_at_ten_experience() {
    let (mut h, storage, id) = villager(&[], [2.5, 64.0, 0.5], "minecraft:become_armorer");
    let (window, offers) = open(&mut h, id);
    let coal = &offers[0];
    assert_eq!((coal.get("buyA").and_then(|buy| buy.get("Name")), int(coal, "buyCountA"), int(coal, "maxUses"), int(coal, "tier")), (Some(&Value::String(COAL.into())), 15, 16, 0));
    assert!(offers.iter().any(|offer| int(offer, "tier") == 4), "every tier listed");
    pay(&mut h, 0, 64);
    let (mut experience, mut tiers, mut orbs) = (Vec::new(), Vec::new(), 0);
    for done in 0..5 {
        if done == 4 {
            pay(&mut h, 1, 11);
        }
        let mut sent = trade(&mut h, int(coal, "netId"));
        (0..3).for_each(|_| sent.extend(h.tick(1)));
        experience.extend(metadata(&sent, id, Key::TradeExperience));
        tiers.extend(metadata(&sent, id, Key::TradeTier));
        orbs += all::<AddEntity>(&sent).iter().filter(|added| added.entity_type == "minecraft:xp_orb").count();
    }
    assert_eq!((experience, tiers), (vec![2, 4, 6, 8, 10], vec![1]));
    assert!(orbs >= 5, "orbs for each trade: {orbs}");
    let closed = close(&mut h, window);
    assert_eq!(all::<ContainerClose>(&closed).len(), 1, "the close is answered");

    h.shard.save_worlds();
    let column = storage.load_column(ChunkPos::new(0, 0), 0).unwrap().expect("the column");
    let kept = sapling_world::stored_actors(&column.actors).unwrap().into_iter().find(|(actor, ..)| actor.as_ref().is_some_and(|actor| actor.identifier == "minecraft:villager_v2"));
    let compound = acacia_proto::nbt::read::<acacia_proto::nbt::LittleEndian>(&mut &kept.expect("the villager kept").2[..]).unwrap().value;
    assert_eq!([compound.get("Variant"), compound.get("TradeTier"), compound.get("TradeExperience")], [Some(&Value::Int(8)), Some(&Value::Int(1)), Some(&Value::Int(10))]);
    let Some(Value::List(recipes)) = compound.get("Offers").and_then(|offers| offers.get("Recipes")) else { panic!("no offers kept") };
    assert_eq!(int(&recipes.items[0], "uses"), 5);
}

/// bds-n1drop.txt: a summoned armorer's first data says `Variant` 8; sapling's said 0 after it.
#[test]
fn a_new_armorer_is_last_told_as_an_armorer() {
    let storage = floor_holding(Vec::new(), &[]);
    let mut h = Harness::with(config(), World::with_storage(storage));
    enter_stored(&mut h, 1);
    h.remote.run(|ctx| assert!(ctx.summon_as(None, "minecraft:villager_v2", [2.5, 64.0, 0.5], 0.0, 0.0, Some("minecraft:become_armorer")).is_some()));
    let sent: Vec<_> = (0..3).flat_map(|_| h.tick(1)).collect();
    let id = all::<AddEntity>(&sent).iter().find(|added| added.entity_type == "minecraft:villager_v2").expect("shown").runtime_id;
    assert_eq!(metadata(&sent, id, Key::Variant).last(), Some(&8));
}

#[test]
fn a_village_golem_is_kept_with_its_group_as_bds_keeps_it() {
    let (mut h, storage, _) = villager(&[], [2.5, 64.0, 0.5], "minecraft:become_armorer");
    h.remote.run(|ctx| assert!(ctx.summon_as(None, "minecraft:iron_golem", [4.5, 64.0, 4.5], 0.0, 0.0, Some("minecraft:from_village")).is_some()));
    (0..2).for_each(|_| drop(h.tick(1)));
    h.shard.save_worlds();
    let column = storage.load_column(ChunkPos::new(0, 0), 0).unwrap().expect("the column");
    let kept = sapling_world::stored_actors(&column.actors).unwrap().into_iter().find(|(actor, ..)| actor.as_ref().is_some_and(|actor| actor.identifier == "minecraft:iron_golem"));
    let groups = sapling_world::actor_groups(&kept.expect("the golem kept").2);
    assert!(groups.iter().any(|group| group == "minecraft:village_created"), "{groups:?}");
}

#[test]
fn a_hero_of_the_village_pays_less_by_its_level() {
    let (mut h, _, id) = villager(&[], [2.5, 64.0, 0.5], "minecraft:become_armorer");
    h.remote.run(|ctx| assert!(ctx.give_effect(sapling_api::PlayerId(1), sapling_api::Effect::HeroOfTheVillage, 48000, 1, false)));
    h.tick(1);
    let (_, offers) = open(&mut h, id);
    assert_eq!(int(&offers[0], "buyCountA"), 10, "15 less floor(15 * 0.3625)");
}

#[test]
fn an_armorer_restocks_at_its_first_start_of_work_in_a_work_period() {
    let (mut h, _, id) = villager(&[([4, 64, 0], FURNACE)], [2.5, 64.0, 2.5], "minecraft:become_armorer");
    (0..100).for_each(|_| drop(h.tick(1)));
    let used_after = |h: &mut Harness, ticks: usize| {
        let (window, offers) = open(h, id);
        pay(h, 0, 15);
        trade(h, int(&offers[0], "netId"));
        close(h, window);
        (0..ticks).for_each(|_| drop(h.tick(1)));
        let (window, offers) = open(h, id);
        close(h, window);
        int(&offers[0], "uses")
    };
    assert_eq!(used_after(&mut h, 500), 1, "its first arrival came before the trade (bds-n1restock.txt)");
    h.remote.run(|ctx| ctx.set_time(10_000));
    assert_eq!(used_after(&mut h, 500), 0, "restocked on arriving in the next work period");
}

/// bds-f3convert.txt: on hard a zombie's killing hit turned an armorer into a zombie villager in
/// the hit's tick, keeping its `Variant` 8 and its tiers; on easy it just dies.
#[test]
fn a_villager_a_zombie_kills_on_hard_becomes_a_zombie_villager_keeping_its_profession() {
    for (difficulty, turns) in [(sapling_api::Difficulty::Hard, true), (sapling_api::Difficulty::Easy, false)] {
        let storage = floor_holding(Vec::new(), &[]);
        let mut h = Harness::with(config(), World::with_storage(storage));
        enter_stored(&mut h, 1);
        h.remote.run(|ctx| assert!(ctx.summon_as(None, "minecraft:villager_v2", [2.5, 64.0, 0.5], 0.0, 0.0, Some("minecraft:become_armorer")).is_some()));
        let sent: Vec<_> = (0..3).flat_map(|_| h.tick(1)).collect();
        let id = all::<AddEntity>(&sent).iter().find(|added| added.entity_type == "minecraft:villager_v2").expect("shown").runtime_id;
        h.remote.run(move |ctx| {
            ctx.set_difficulty(difficulty);
            let zombie = sapling_api::Attacker::Mob { kind: "minecraft:zombie".into() };
            assert!(ctx.hurt_entity_by(sapling_api::EntityId(id), 100.0, sapling_api::DamageCause::Attack, &zombie));
        });
        let sent = h.tick(1);
        let added: Vec<AddEntity> = all::<AddEntity>(&sent).into_iter().filter(|added| added.entity_type == "minecraft:zombie_villager_v2").collect();
        assert_eq!(added.len(), usize::from(turns), "{difficulty:?}");
        if let Some(zombie) = added.first() {
            assert_eq!(metadata(&sent, zombie.runtime_id, Key::Variant).last(), Some(&8));
            assert_eq!(metadata(&sent, zombie.runtime_id, Key::MaxTradeTier).last(), Some(&4));
        }
    }
}

/// bds-f3cure.txt: a weakened zombie villager given a golden apple shook, was heard `remedy` a tick
/// later from its middle, and became a villager 178 to 211 s on, heard `unfect` (BDS 1424459c0: 180
/// to 300 s less what beds and iron bars took off); the apple is used up.
#[test]
fn a_weakened_zombie_villager_given_a_golden_apple_shakes_and_is_cured() {
    use acacia_proto::packets::LevelSoundEvent;
    use acacia_proto::types::MetadataFlags1;
    let storage = floor_holding(Vec::new(), &[]);
    let mut h = Harness::with(config(), World::with_storage(storage));
    enter_stored(&mut h, 1);
    h.remote.run(|ctx| {
        ctx.set_difficulty(sapling_api::Difficulty::Easy);
        ctx.set_game_mode(sapling_api::PlayerId(1), sapling_api::GameMode::Survival);
        assert!(ctx.set_item(sapling_api::PlayerId(1), sapling_api::Slot::Main(0), Some(&sapling_api::Item::new("minecraft:golden_apple", 2))));
        assert!(ctx.summon_as(None, "minecraft:zombie_villager_v2", [2.5, 64.0, 0.5], 0.0, 0.0, None).is_some());
    });
    let sent: Vec<_> = (0..3).flat_map(|_| h.tick(1)).collect();
    let zombie = all::<AddEntity>(&sent).iter().find(|added| added.entity_type == "minecraft:zombie_villager_v2").expect("shown").runtime_id;
    h.remote.run(move |ctx| {
        for effect in [sapling_api::Effect::Weakness, sapling_api::Effect::FireResistance] {
            assert!(ctx.give_entity_effect(sapling_api::EntityId(zombie), effect, 20_000, 0));
        }
    });
    h.tick(1);
    h.receive(1, &use_on(zombie));
    let sent = h.tick(1);
    assert_eq!(common::raising::has_flag(&sent, zombie, MetadataFlags1::VIBRATING), Some(true), "it shakes");
    let remedy = all::<LevelSoundEvent>(&h.tick(1)).into_iter().find(|sound| sound.sound_id == "remedy").expect("remedy heard a tick on");
    assert!((remedy.position.y - 64.95).abs() < 1e-4 && remedy.entity_unique_id == -1, "from its middle, of no entity");
    let left = common::asking::ask(&mut h, |ctx| ctx.item(sapling_api::PlayerId(1), sapling_api::Slot::Main(0)).map(|item| item.count));
    assert_eq!(left, Some(1), "the apple used up");
    let cured = (1..=6000).find_map(|tick| {
        let sent = h.tick(1);
        let villager = all::<AddEntity>(&sent).iter().any(|added| added.entity_type == "minecraft:villager_v2");
        villager.then(|| (tick, all::<LevelSoundEvent>(&sent).iter().any(|sound| sound.sound_id == "unfect")))
    });
    let (tick, heard) = cured.expect("cured within 300 s");
    assert!(tick >= 3600, "not before 180 s: {tick}");
    assert!(heard, "unfect heard");
}
