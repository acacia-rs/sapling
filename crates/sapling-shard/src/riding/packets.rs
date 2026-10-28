//! What riders' viewers are told, as BDS told them (research/captures/bds-ride.txt): the rider's
//! seat in its metadata, then the "mount" sound and the link; a link of type 0 when it leaves.

use acacia_proto::packets::{LevelSoundEvent, SetEntityData, SetEntityLink};
use acacia_proto::types::{
    EntityProperties, Link, MetadataDictionaryItem as Item, MetadataFlags1, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value,
    MetadataDictionaryItemValueDefault as Plain, Vec3f,
};
use bytes::Bytes;

use super::seats::Seat;

/// A rider without a lock may turn this far: all the way.
const FREE_TURN: f32 = 181.0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum LinkKind {
    Off = 0,
    Driver = 1,
    Passenger = 2,
}

pub(super) fn link(vehicle: u64, rider: u64, kind: LinkKind, immediate: bool) -> Bytes {
    let link = Link { ridden_entity_id: vehicle as i64, rider_entity_id: rider as i64, r#type: kind as u8, immediate, rider_initiated: true, angular_velocity: 0.0 };
    sapling_net::encode(&SetEntityLink { link })
}

fn item(key: Key, value: Plain) -> Item {
    let (r#type, legacy_type) = match value {
        Plain::Byte(_) => (Type::Byte, 0),
        Plain::Int(_) => (Type::Int, 2),
        Plain::Float(_) => (Type::Float, 3),
        Plain::String(_) => (Type::String, 4),
        _ => (Type::Vec3f, 8),
    };
    Item { key, r#type, legacy_type, value: Value::Default(value) }
}

fn data(id: u64, metadata: Vec<Item>) -> Bytes {
    sapling_net::encode(&SetEntityData { runtime_entity_id: id, metadata, properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 })
}

/// The llama `id`'s strength and the most it may have.
pub(super) fn strength(id: u64, strength: i32, max: i32) -> Bytes {
    data(id, vec![item(Key::Strength, Plain::Int(strength)), item(Key::MaxStrength, Plain::Int(max))])
}

/// The rider `id`'s seat, `position` against its vehicle; with its turn held, all of BDS's items.
pub(super) fn seat(id: u64, seat: &Seat, position: [f32; 3], whole: bool) -> Bytes {
    let [x, y, z] = position;
    let mut metadata = vec![item(Key::RiderSeatPosition, Plain::Vec3f(Vec3f { x, y, z }))];
    if !whole {
        return data(id, metadata);
    }
    match seat.lock {
        Some(lock) => {
            metadata.extend([item(Key::RiderRotationLocked, Plain::Byte(1)), item(Key::RiderMaxRotation, Plain::Float(lock)), item(Key::RiderMinRotation, Plain::Byte(1))]);
            if seat.turn != 0.0 {
                metadata.push(item(Key::RiderSeatRotationOffset, Plain::Float(seat.turn)));
            }
        }
        None => metadata.push(item(Key::RiderMaxRotation, Plain::Float(FREE_TURN))),
    }
    data(id, metadata)
}

/// The whole word of an entity's flags.
pub(super) fn flags(id: u64, word: MetadataFlags1) -> Bytes {
    data(id, vec![Item { key: Key::Flags, r#type: Type::Long, legacy_type: 7, value: Value::Flags(word) }])
}

/// The sound `name` ("mount", "saddle") of `entity` of `kind` at `at`.
pub(crate) fn sound(name: &str, entity: u64, kind: &str, [x, y, z]: [f32; 3]) -> Bytes {
    let sound = LevelSoundEvent {
        sound_id: name.into(),
        position: Vec3f { x, y, z },
        extra_data: -1,
        entity_type: kind.into(),
        is_baby_mob: false,
        is_global: false,
        entity_unique_id: entity as i64,
        fire_at_position: None,
    };
    sapling_net::encode(&sound)
}

#[cfg(test)]
mod tests {
    use acacia_proto::RawPacket;

    use super::*;

    #[test]
    fn a_boat_seat_tells_bds_s_five_items_and_a_cart_seat_two() {
        let boat = Seat { position: [0.0, -0.2, 0.0], lock: Some(90.0), turn: -90.0 };
        let told = RawPacket::parse(seat(1, &boat, [0.0, 1.02001, 0.0], true)).unwrap().decode::<SetEntityData>().unwrap();
        let keys: Vec<Key> = told.metadata.iter().map(|item| item.key.clone()).collect();
        assert_eq!(keys, [Key::RiderSeatPosition, Key::RiderRotationLocked, Key::RiderMaxRotation, Key::RiderMinRotation, Key::RiderSeatRotationOffset]);
        let cart = Seat { position: [0.0, -0.2, 0.0], lock: None, turn: 0.0 };
        assert_eq!(RawPacket::parse(seat(1, &cart, [0.0; 3], true)).unwrap().decode::<SetEntityData>().unwrap().metadata.len(), 2);
    }
}
