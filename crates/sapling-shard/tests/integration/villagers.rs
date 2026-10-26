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
    h.remote.run(move |ctx| assert!(ctx.summon_as(None, "minecraft:villager_v2", feet, 0.0, Some(event)).is_some()));
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

#[test]
fn an_armorer_restocks_at_its_job_site_once_a_work_period() {
    let (mut h, _, id) = villager(&[([4, 64, 0], FURNACE)], [2.5, 64.0, 2.5], "minecraft:become_armorer");
    (0..100).for_each(|_| drop(h.tick(1)));
    let used_after = |h: &mut Harness| {
        let (window, offers) = open(h, id);
        pay(h, 0, 15);
        trade(h, int(&offers[0], "netId"));
        close(h, window);
        (0..120).for_each(|_| drop(h.tick(1)));
        let (window, offers) = open(h, id);
        close(h, window);
        int(&offers[0], "uses")
    };
    assert_eq!(used_after(&mut h), 0, "restocked on reaching its blast furnace");
    assert_eq!(used_after(&mut h), 1, "not twice in one work period");
}
