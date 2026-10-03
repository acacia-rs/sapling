//! The joining player's own state as BDS sends it to a fresh survival player, re-addressed.

use acacia_proto::packets::{SetEntityData, UpdateAbilities, UpdateAttributes};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Default};

use crate::templates::{SET_ENTITY_DATA, UPDATE_ABILITIES, UPDATE_ATTRIBUTES, decode};

/// Health, hunger, movement speed and the rest of a fresh player's attributes.
pub fn update_attributes(runtime_entity_id: u64) -> UpdateAttributes {
    UpdateAttributes { runtime_entity_id, ..decode(UPDATE_ATTRIBUTES) }
}

/// Member permissions with the default ability layer (build, mine, doors, attack).
pub fn update_abilities(entity_unique_id: i64) -> UpdateAbilities {
    UpdateAbilities { entity_unique_id, ..decode(UPDATE_ABILITIES) }
}

/// The player's entity metadata (gravity, collision and breathing flags, hitbox), named `name`.
pub fn entity_data(runtime_entity_id: u64, name: &str) -> SetEntityData {
    let mut data = SetEntityData { runtime_entity_id, ..decode(SET_ENTITY_DATA) };
    for item in data.metadata.iter_mut().filter(|i| i.key == Key::Nametag) {
        item.value = Value::Default(Default::String(name.to_owned()));
    }
    data
}

#[cfg(test)]
mod tests {
    use acacia_proto::Packet;
    use bytes::BytesMut;

    use super::*;

    fn roundtrip<T: Packet>(packet: &T) -> T {
        let mut out = BytesMut::new();
        packet.encode(&mut out);
        T::decode(&mut &out[..]).unwrap()
    }

    #[test]
    fn player_packets_are_readdressed() {
        assert_eq!(roundtrip(&update_attributes(9)).runtime_entity_id, 9);
        assert!(update_attributes(9).attributes.iter().any(|a| a.name == "minecraft:movement"));
        assert_eq!(roundtrip(&update_abilities(-4)).entity_unique_id, -4);
        let data = roundtrip(&entity_data(9, "Steve"));
        assert_eq!(data.runtime_entity_id, 9);
        let name = data.metadata.iter().find(|i| i.key == Key::Nametag).expect("a nametag");
        assert_eq!(name.value, Value::Default(Default::String("Steve".into())));
    }
}
