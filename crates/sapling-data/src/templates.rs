//! Packet bodies recorded from BDS (`data/`, regenerate with `tools/extract-data`).

use std::sync::LazyLock;

use acacia_proto::packets::{
    AvailableEntityIdentifiers, BiomeDefinitionList, CameraPresets, CraftingData, CreativeContent, FeatureRegistry, ItemRegistry,
    JigsawStructureData, TrimData, VoxelShapes,
};
use acacia_proto::{Packet, codec};
use bytes::{Bytes, BytesMut};

pub(crate) const START_GAME: &[u8] = include_bytes!("../data/StartGame.bin");
pub(crate) const UPDATE_ATTRIBUTES: &[u8] = include_bytes!("../data/UpdateAttributes.bin");
pub(crate) const UPDATE_ABILITIES: &[u8] = include_bytes!("../data/UpdateAbilities.bin");
pub(crate) const SET_ENTITY_DATA: &[u8] = include_bytes!("../data/SetEntityData.bin");
pub(crate) const ITEM_REGISTRY: &[u8] = include_bytes!("../data/ItemRegistry.bin");
pub(crate) const CREATIVE_CONTENT: &[u8] = include_bytes!("../data/CreativeContent.bin");
pub(crate) const CRAFTING_DATA: &[u8] = include_bytes!("../data/CraftingData.bin");
pub(crate) const TRIM_DATA: &[u8] = include_bytes!("../data/TrimData.bin");
pub(crate) const UNLOCKED_RECIPES: &[u8] = include_bytes!("../data/UnlockedRecipes.bin");

pub(crate) fn decode<T: Packet>(body: &[u8]) -> T {
    T::decode(&mut &body[..]).unwrap_or_else(|e| panic!("bundled {} template: {e}", T::NAME))
}

/// The bodies in a file of several recordings, each behind its length (u32, little-endian).
pub(crate) fn records(mut file: &'static [u8]) -> impl Iterator<Item = &'static [u8]> {
    std::iter::from_fn(move || {
        let (length, rest) = file.split_first_chunk::<4>()?;
        let (body, rest) = rest.split_at(u32::from_le_bytes(*length) as usize);
        file = rest;
        Some(body)
    })
}

fn framed<T: Packet>(body: &[u8]) -> Bytes {
    let mut out = BytesMut::with_capacity(body.len() + 2);
    codec::write_varint(&mut out, T::ID);
    out.extend_from_slice(body);
    out.freeze()
}

/// `packet` with its header, as the registries are kept.
pub(crate) fn encoded<T: Packet>(packet: &T) -> Bytes {
    let mut body = BytesMut::new();
    packet.encode(&mut body);
    framed::<T>(&body)
}

/// What BDS sends before StartGame. Without JigsawStructureData the vanilla client refuses to
/// enter the world ("missing structure data").
pub fn pre_start_packets() -> &'static [Bytes] {
    static PACKETS: LazyLock<[Bytes; 2]> = LazyLock::new(|| {
        [
            framed::<JigsawStructureData>(include_bytes!("../data/JigsawStructureData.bin")),
            framed::<VoxelShapes>(include_bytes!("../data/VoxelShapes.bin")),
        ]
    });
    &*PACKETS
}

/// The registries as BDS sent them, each with its header, in the order BDS sends them after StartGame,
/// with the custom items in ItemRegistry and CreativeContent (custom_creative.rs).
pub fn registry_packets() -> &'static [Bytes] {
    let generation = crate::custom_items::generation();
    if generation == 0 {
        return recorded_registry_packets();
    }
    static WITH_CUSTOM: std::sync::Mutex<(usize, &'static [Bytes])> = std::sync::Mutex::new((0, &[]));
    let mut cached = WITH_CUSTOM.lock().expect("registry packets lock");
    if cached.0 != generation {
        let mut packets = recorded_registry_packets().to_vec();
        packets[0] = encoded(&crate::custom_creative::item_registry());
        packets[5] = encoded(&crate::custom_creative::creative_content());
        // Leaked once per registration, which happens before a server starts.
        *cached = (generation, Vec::leak(packets));
    }
    cached.1
}

fn recorded_registry_packets() -> &'static [Bytes] {
    static PACKETS: LazyLock<[Bytes; 8]> = LazyLock::new(|| {
        [
            framed::<ItemRegistry>(ITEM_REGISTRY),
            framed::<BiomeDefinitionList>(include_bytes!("../data/BiomeDefinitionList.bin")),
            framed::<AvailableEntityIdentifiers>(include_bytes!("../data/AvailableEntityIdentifiers.bin")),
            framed::<FeatureRegistry>(include_bytes!("../data/FeatureRegistry.bin")),
            framed::<CameraPresets>(include_bytes!("../data/CameraPresets.bin")),
            framed::<CreativeContent>(CREATIVE_CONTENT),
            framed::<TrimData>(TRIM_DATA),
            // BDS sends the recipes after the player's own windows; here they go with the rest.
            // The player's recipe book follows once its items are shown (sapling-shard recipe_book.rs).
            framed::<CraftingData>(CRAFTING_DATA),
        ]
    });
    &*PACKETS
}

#[cfg(test)]
mod tests {
    use acacia_proto::RawPacket;

    use super::*;

    #[test]
    fn registry_packets_decode_in_bds_order() {
        let ids: Vec<u32> = registry_packets().iter().map(|p| RawPacket::parse(p.clone()).unwrap().id).collect();
        let expected = [
            ItemRegistry::ID,
            BiomeDefinitionList::ID,
            AvailableEntityIdentifiers::ID,
            FeatureRegistry::ID,
            CameraPresets::ID,
            CreativeContent::ID,
            TrimData::ID,
            CraftingData::ID,
        ];
        assert_eq!(ids, expected);
        let raw = |i: usize| RawPacket::parse(registry_packets()[i].clone()).unwrap();
        let items: ItemRegistry = raw(0).decode().unwrap();
        let shield = items.itemstates.iter().find(|i| i.name == "minecraft:shield").expect("a shield item");
        raw(1).decode::<BiomeDefinitionList>().unwrap();
        raw(2).decode::<AvailableEntityIdentifiers>().unwrap();
        // Item stacks only decode once the registry's shield id is known (acacia-proto docs/proto.md).
        acacia_proto::manual::set_shield_item_id(shield.runtime_id.into());
        raw(5).decode::<CreativeContent>().unwrap();
        let before: Vec<u32> = pre_start_packets().iter().map(|p| RawPacket::parse(p.clone()).unwrap().id).collect();
        assert_eq!(before, [JigsawStructureData::ID, VoxelShapes::ID]);
    }
}
