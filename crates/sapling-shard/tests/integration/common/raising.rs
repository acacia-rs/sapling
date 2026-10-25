//! Clicks on mobs and what their viewers are told of them, for the husbandry tests.

use std::sync::{Arc, Mutex};

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, EntityEvent, EntityEventEventId, InventoryTransaction, SetEntityData};
use acacia_proto::types::{
    MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as MetaValue, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1,
    TransactionTransactionData, TransactionTransactionDataItemUseOnEntityActionType as OnEntity,
};

use sapling_api::EntityId;

use super::{Harness, all, attack};

/// A click (a use, not a hit) on the entity `id`.
pub fn use_on(id: u64) -> InventoryTransaction {
    let mut used = attack(id);
    if let TransactionTransactionData::ItemUseOnEntity(on) = &mut used.transaction.transaction_data {
        on.action_type = OnEntity::Interact;
    }
    used
}

/// Alex (conn 1) clicks the entity `id` with what it holds; what Alex was told that tick.
pub fn click_on(h: &mut Harness, id: u64) -> Vec<RawPacket> {
    h.receive(1, &use_on(id));
    h.tick_all().to(1).to_vec()
}

fn flags_in(metadata: &[MetadataDictionaryItem]) -> Option<i64> {
    metadata.iter().find_map(|item| if let MetaValue::Flags(flags) = item.value { Some(flags.0) } else { None })
}

/// The value of `key` in `metadata`.
pub fn plain_in(metadata: &[MetadataDictionaryItem], key: Key) -> Option<Plain> {
    metadata.iter().find(|item| item.key == key).and_then(|item| if let MetaValue::Default(plain) = &item.value { Some(plain.clone()) } else { None })
}

/// Whether the entity `id` was added or set in `sent` with `flag` on, last.
pub fn has_flag(sent: &[RawPacket], id: u64, flag: MetadataFlags1) -> Option<bool> {
    let added = all::<AddEntity>(sent).into_iter().filter(|added| added.runtime_id == id).filter_map(|added| flags_in(&added.metadata));
    let set = all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == id).filter_map(|data| flags_in(&data.metadata));
    added.chain(set).last().map(|flags| flags & flag.0 != 0)
}

/// The last value of `key` the entity `id` was set to in `sent`.
pub fn set_in(sent: &[RawPacket], id: u64, key: Key) -> Option<Plain> {
    all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == id).filter_map(|data| plain_in(&data.metadata, key)).last()
}

/// Where the entity `id` stands now.
pub fn feet_of(h: &mut Harness, id: u64) -> [f32; 3] {
    let found = Arc::new(Mutex::new(None));
    let out = found.clone();
    h.remote.run(move |ctx| *out.lock().unwrap() = ctx.entity(EntityId(id)).map(|info| info.feet));
    h.tick_all();
    found.lock().unwrap().take().expect("the entity")
}

/// Whether the entity `id` showed `event` in `sent`.
pub fn showed(sent: &[RawPacket], id: u64, event: EntityEventEventId) -> bool {
    all::<EntityEvent>(sent).iter().any(|shown| shown.runtime_entity_id == id && shown.event_id == event)
}
