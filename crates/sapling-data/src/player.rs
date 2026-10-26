//! A player's state as BDS sends it for a fresh survival player, re-addressed: to the player
//! itself at join, and to the players who see it (AddPlayer shape: `sapling` example `bds_probe`).

use acacia_proto::manual::Uuid;
use acacia_proto::packets::{AddPlayer, SetEntityData, UpdateAbilities, UpdateAttributes};
use acacia_proto::types::{
    DeviceOS, GameMode, ItemV4, MetadataDictionary, MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value,
    MetadataDictionaryItemValueDefault as Default, MetadataFlags1, MetadataFlags2, Vec3f,
};

use crate::templates::{SET_ENTITY_DATA, UPDATE_ABILITIES, UPDATE_ATTRIBUTES, decode};

/// Health, hunger, movement speed and the rest of a fresh player's attributes.
pub fn update_attributes(runtime_entity_id: u64) -> UpdateAttributes {
    UpdateAttributes { runtime_entity_id, ..decode(UPDATE_ATTRIBUTES) }
}

/// How far a player is into its next level (0 to 1), and its level when that changed, in the
/// recorded attributes' shape (`minecraft:player.experience`, `minecraft:player.level` up to
/// 24791), at server tick `tick`. BDS sends only the progress when the level stays, and names each
/// twice (REC: `bds_container_probe`, research/captures/containers-findings.md D5).
pub fn experience_attributes(runtime_entity_id: u64, level: Option<i32>, progress: f32, tick: u64) -> UpdateAttributes {
    const LEVEL: &str = "minecraft:player.level";
    const PROGRESS: &str = "minecraft:player.experience";
    const TIMES: usize = 2;
    let mut packet = update_attributes(runtime_entity_id);
    let mut seen: Vec<String> = Vec::new();
    packet.attributes.retain(|attribute| {
        let wanted = attribute.name == PROGRESS || attribute.name == LEVEL && level.is_some();
        let fits = seen.iter().filter(|name| **name == attribute.name).count() < TIMES;
        seen.push(attribute.name.clone());
        wanted && fits
    });
    for attribute in &mut packet.attributes {
        attribute.current = match level {
            Some(level) if attribute.name == LEVEL => level as f32,
            _ => progress,
        };
    }
    UpdateAttributes { tick, ..packet }
}

/// Member permissions with the default ability layer (build, mine, doors, attack).
pub fn update_abilities(entity_unique_id: i64) -> UpdateAbilities {
    UpdateAbilities { entity_unique_id, ..decode(UPDATE_ABILITIES) }
}

/// A player's entity metadata (gravity and collision flags, hitbox), named `name`, with `flags`
/// set on top of the recorded ones.
pub fn player_metadata(name: &str, flags: MetadataFlags1) -> MetadataDictionary {
    let mut metadata = decode::<SetEntityData>(SET_ENTITY_DATA).metadata;
    for item in &mut metadata {
        match &mut item.value {
            value if item.key == Key::Nametag => *value = Value::Default(Default::String(name.to_owned())),
            Value::Flags(recorded) => recorded.insert(flags),
            _ => {}
        }
    }
    metadata
}

/// The `FlagsExtended` BDS keeps on every player, as recorded.
pub fn player_flags_extended() -> MetadataFlags2 {
    let metadata = decode::<SetEntityData>(SET_ENTITY_DATA).metadata;
    metadata.into_iter().find_map(|item| if let Value::FlagsExtended(flags) = item.value { Some(flags) } else { None }).unwrap_or_default()
}

/// The player's own metadata, as sent to it at join.
pub fn entity_data(runtime_entity_id: u64, name: &str) -> SetEntityData {
    SetEntityData { runtime_entity_id, metadata: player_metadata(name, MetadataFlags1(0)), ..decode(SET_ENTITY_DATA) }
}

/// Who appears where, for [`add_player`].
pub struct AddPlayerOptions<'a> {
    pub uuid: Uuid,
    pub name: &'a str,
    /// Both the runtime and the unique entity id.
    pub id: u64,
    pub feet: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub head_yaw: f32,
    pub flags: MetadataFlags1,
    pub device_id: &'a str,
    /// `DeviceOS` of the player's client data.
    pub device_os: i32,
    /// What is in the player's hand ([`crate::no_item`] for nothing).
    pub held: ItemV4,
}

/// A player entering someone's view. The position is the feet, unlike MovePlayer's.
pub fn add_player(o: &AddPlayerOptions<'_>) -> AddPlayer {
    let abilities: UpdateAbilities = decode(UPDATE_ABILITIES);
    let [x, y, z] = o.feet;
    AddPlayer {
        uuid: o.uuid,
        username: o.name.to_owned(),
        runtime_id: o.id,
        platform_chat_id: String::new(),
        position: Vec3f { x, y, z },
        velocity: Vec3f { x: 0.0, y: 0.0, z: 0.0 },
        pitch: o.pitch,
        yaw: o.yaw,
        head_yaw: o.head_yaw,
        held_item: o.held.clone(),
        // BDS sends Fallback: the viewer shows the world's game mode.
        gamemode: GameMode::Fallback,
        metadata: player_metadata(o.name, o.flags),
        properties: decode::<SetEntityData>(SET_ENTITY_DATA).properties,
        unique_id: o.id as i64,
        permission_level: abilities.permission_level,
        command_permission: abilities.command_permission,
        abilities: abilities.abilities,
        links: Vec::new(),
        device_id: o.device_id.to_owned(),
        device_os: Some(DeviceOS::from_raw(o.device_os.into())),
    }
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

    #[test]
    fn a_player_is_added_as_bds_adds_one() {
        let uuid: Uuid = "4c8ae710-df2e-47cd-814d-cc7bf21a3d67".parse().unwrap();
        let options = AddPlayerOptions {
            uuid,
            name: "Alex",
            id: 7,
            feet: [1.5, 64.0, -2.5],
            yaw: 90.0,
            pitch: 10.0,
            head_yaw: 95.0,
            flags: MetadataFlags1::SNEAKING,
            device_id: "device",
            device_os: 1,
            held: crate::no_item(),
        };
        let added = roundtrip(&add_player(&options));
        assert_eq!((added.uuid, added.username.as_str(), added.runtime_id, added.unique_id), (uuid, "Alex", 7, 7));
        assert_eq!((added.position.y, added.yaw, added.pitch, added.head_yaw), (64.0, 90.0, 10.0, 95.0));
        assert_eq!((added.gamemode, added.device_os, added.held_item.network_id), (GameMode::Fallback, Some(DeviceOS::Android), 0));
        let flags = added.metadata.iter().find_map(|i| if let Value::Flags(f) = &i.value { Some(*f) } else { None }).expect("flags");
        // The recorded flags (collision, gravity, climbing) stay; sneaking is added.
        assert_eq!(flags.0, 844_424_930_656_256 | MetadataFlags1::SNEAKING.0);
        assert_eq!(added.abilities, update_abilities(7).abilities);
        assert_eq!(added.metadata.len(), entity_data(7, "Alex").metadata.len());
    }
}
