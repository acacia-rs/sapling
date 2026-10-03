//! Packet bodies recorded from BDS (`data/`, regenerate with `tools/extract-data`).

use std::sync::LazyLock;

use acacia_proto::packets::{
    AvailableEntityIdentifiers, BiomeDefinitionList, CameraPresets, CreativeContent, FeatureRegistry, ItemRegistry, JigsawStructureData,
    TrimData, VoxelShapes,
};
use acacia_proto::{Packet, codec};
use bytes::{Bytes, BytesMut};

pub(crate) const START_GAME: &[u8] = include_bytes!("../data/StartGame.bin");
pub(crate) const UPDATE_ATTRIBUTES: &[u8] = include_bytes!("../data/UpdateAttributes.bin");
pub(crate) const UPDATE_ABILITIES: &[u8] = include_bytes!("../data/UpdateAbilities.bin");
pub(crate) const SET_ENTITY_DATA: &[u8] = include_bytes!("../data/SetEntityData.bin");

pub(crate) fn decode<T: Packet>(body: &[u8]) -> T {
    T::decode(&mut &body[..]).unwrap_or_else(|e| panic!("bundled {} template: {e}", T::NAME))
}

fn framed<T: Packet>(body: &[u8]) -> Bytes {
    let mut out = BytesMut::with_capacity(body.len() + 2);
    codec::write_varint(&mut out, T::ID);
    out.extend_from_slice(body);
    out.freeze()
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

/// The registries as BDS sent them, each with its header, in the order BDS sends them after StartGame.
pub fn registry_packets() -> &'static [Bytes] {
    static PACKETS: LazyLock<[Bytes; 7]> = LazyLock::new(|| {
        [
            framed::<ItemRegistry>(include_bytes!("../data/ItemRegistry.bin")),
            framed::<BiomeDefinitionList>(include_bytes!("../data/BiomeDefinitionList.bin")),
            framed::<AvailableEntityIdentifiers>(include_bytes!("../data/AvailableEntityIdentifiers.bin")),
            framed::<FeatureRegistry>(include_bytes!("../data/FeatureRegistry.bin")),
            framed::<CameraPresets>(include_bytes!("../data/CameraPresets.bin")),
            framed::<CreativeContent>(include_bytes!("../data/CreativeContent.bin")),
            framed::<TrimData>(include_bytes!("../data/TrimData.bin")),
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
