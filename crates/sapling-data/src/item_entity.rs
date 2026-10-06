//! Item stacks on entities: a dropped item entering someone's view (metadata as BDS gave one,
//! `data/ItemEntity.bin`, README "Mobs") and what a mob wears and holds (shapes from
//! `bds_entity_probe` on a stored armour stand).

use std::sync::LazyLock;

use acacia_proto::packets::{AddItemEntity, MobArmorEquipment, MobEquipment};
use acacia_proto::types::{ItemV4, Vec3f, WindowID};

use crate::items::{Stack, no_item as nothing};
use crate::templates::decode;

static TEMPLATE: LazyLock<AddItemEntity> = LazyLock::new(|| decode(include_bytes!("../data/ItemEntity.bin")));

fn item(stack: &Stack<'_>) -> Option<ItemV4> {
    crate::items::item(stack, None)
}

/// A stack at `position` moving at `velocity`, under `id` as both runtime and unique id; `None`
/// for an item the registry does not list.
pub fn add_item_entity(id: u64, stack: &Stack<'_>, position: [f32; 3], velocity: [f32; 3]) -> Option<AddItemEntity> {
    let ([x, y, z], [dx, dy, dz]) = (position, velocity);
    Some(AddItemEntity {
        entity_id_self: id as i64,
        runtime_entity_id: id,
        item: item(stack)?,
        position: Vec3f { x, y, z },
        velocity: Vec3f { x: dx, y: dy, z: dz },
        is_from_fishing: false,
        ..TEMPLATE.clone()
    })
}

/// What a mob wears: helmet, chestplate, leggings, boots. A slot that is empty, missing, or holds
/// an item the registry does not list shows nothing.
pub fn mob_armor(runtime_entity_id: u64, slots: &[Option<Stack<'_>>]) -> MobArmorEquipment {
    let slot = |index: usize| slots.get(index).and_then(Option::as_ref).and_then(item).unwrap_or_else(nothing);
    MobArmorEquipment { runtime_entity_id, helmet: slot(0), chestplate: slot(1), leggings: slot(2), boots: slot(3), body: nothing() }
}

/// What a mob holds in its main hand, or in the other; `None` for an item the registry does not list.
pub fn mob_hand(runtime_entity_id: u64, stack: &Stack<'_>, offhand: bool) -> Option<MobEquipment> {
    let window_id = if offhand { WindowID::Offhand } else { WindowID::Inventory };
    Some(MobEquipment { runtime_entity_id, item: item(stack)?, slot: 0, selected_slot: 0, window_id })
}

#[cfg(test)]
mod tests {
    use acacia_proto::{Packet, nbt};
    use bytes::BytesMut;

    use super::*;
    use crate::items::{NETWORK_IDS, SHIELD};

    fn stack<'a>(name: &'a str, tag: Option<&'a nbt::Value>) -> Stack<'a> {
        Stack { name, count: 3, damage: 0, block: None, tag }
    }

    fn reread<T: Packet + PartialEq + std::fmt::Debug>(packet: &T) -> T {
        let mut out = BytesMut::new();
        packet.encode(&mut out);
        let read = T::decode(&mut &out[..]).unwrap();
        assert_eq!(&read, packet);
        read
    }

    #[test]
    fn an_item_survives_the_wire_with_its_stack() {
        // What a client does on ItemRegistry; the shield's extra data cannot be read back without.
        acacia_proto::manual::set_shield_item_id(NETWORK_IDS[SHIELD].into());
        let tag = nbt::Value::Compound(vec![("Damage".into(), nbt::Value::Int(4))]);
        for (name, tag) in [("minecraft:diamond", None), ("minecraft:iron_sword", Some(&tag)), (SHIELD, None), (SHIELD, Some(&tag))] {
            let read = reread(&add_item_entity(12, &stack(name, tag), [1.5, 64.0, -2.5], [0.0; 3]).expect("a vanilla item"));
            assert_eq!((read.runtime_entity_id, read.item.count, read.item.network_id), (12, 3, NETWORK_IDS[name]));
            assert!(!read.metadata.is_empty(), "BDS's metadata comes along");
        }
        assert!(add_item_entity(12, &stack("sapling:no_such_item", None), [0.0; 3], [0.0; 3]).is_none());
    }

    #[test]
    fn a_mob_wears_and_holds_what_the_registry_knows() {
        acacia_proto::manual::set_shield_item_id(NETWORK_IDS[SHIELD].into());
        let slots = [Some(stack("minecraft:diamond_helmet", None)), None, Some(stack("sapling:no_such_item", None)), Some(stack("minecraft:golden_boots", None))];
        let armor = reread(&mob_armor(5, &slots));
        let ids = [&armor.helmet, &armor.chestplate, &armor.leggings, &armor.boots, &armor.body].map(|item| item.network_id);
        assert_eq!(ids, [NETWORK_IDS["minecraft:diamond_helmet"], 0, 0, NETWORK_IDS["minecraft:golden_boots"], 0]);
        assert_eq!(armor.chestplate, nothing());
        let held = reread(&mob_hand(5, &stack(SHIELD, None), true).unwrap());
        assert_eq!((held.window_id, held.slot, held.item.network_id), (WindowID::Offhand, 0, NETWORK_IDS[SHIELD]));
        assert_eq!(mob_hand(5, &stack("minecraft:iron_sword", None), false).unwrap().window_id, WindowID::Inventory);
    }
}
