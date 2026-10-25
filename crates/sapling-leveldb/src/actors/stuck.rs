//! The actor a stuck arrow or a lying trident is kept as. The fields are those the Bedrock wiki's
//! entity format names (`player`, `isCreative`, `auxValue`, `Trident`); no BDS record was read.

use acacia_proto::nbt::{self, LittleEndian, Value};
use acacia_world::BlockRegistry;

use super::{floats, written};
use crate::ItemStack;

const PLAYER: &str = "player";
const CREATIVE: &str = "isCreative";
const AUX: &str = "auxValue";
const TRIDENT: &str = "Trident";

/// What a shot keeps beyond any actor's fields: whether a player loosed it, so that it is picked
/// up, or one in creative, so that it is taken without being had; a tipped arrow's potion, one
/// more than its data value; and the trident a thrown one is.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ShotRecord {
    pub player: bool,
    pub creative: bool,
    pub aux: i8,
    pub trident: Option<ItemStack>,
}

/// A new shot actor of `identifier` with the bottom of its box at `position`.
pub fn new_shot_actor(unique_id: i64, identifier: &str, position: [f32; 3], rotation: [f32; 2], shot: &ShotRecord, registry: &BlockRegistry) -> Vec<u8> {
    let mut fields = vec![
        ("UniqueID", Value::Long(unique_id)),
        ("identifier", Value::String(identifier.into())),
        ("Pos", floats(&position)),
        ("Rotation", floats(&rotation)),
        ("Motion", floats(&[0.0; 3])),
        (PLAYER, Value::Byte(i8::from(shot.player))),
        (CREATIVE, Value::Byte(i8::from(shot.creative))),
        (AUX, Value::Byte(shot.aux)),
    ];
    fields.extend(shot.trident.as_ref().map(|trident| (TRIDENT, Value::Compound(trident.fields(registry)))));
    written(Value::Compound(fields.into_iter().map(|(key, value)| (key.into(), value)).collect()))
}

/// What the shot kept as `compound` says of itself.
pub fn shot_record(compound: &[u8]) -> Option<ShotRecord> {
    let actor = nbt::read::<LittleEndian>(&mut &compound[..]).ok()?.value;
    let byte = |key: &str| if let Some(Value::Byte(byte)) = actor.get(key) { *byte } else { 0 };
    Some(ShotRecord { player: byte(PLAYER) != 0, creative: byte(CREATIVE) != 0, aux: byte(AUX), trident: actor.get(TRIDENT).and_then(ItemStack::parse) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actors::actors;

    #[test]
    fn a_stuck_shot_is_read_back_as_it_was_written() {
        let registry = BlockRegistry::vanilla();
        let trident = ItemStack { name: "minecraft:trident".into(), count: 1, damage: 3, block: None, tag: None };
        let shot = ShotRecord { player: true, creative: false, aux: 26, trident: Some(trident) };
        let kept = new_shot_actor(5, "minecraft:thrown_trident", [0.5, 70.0, 0.5], [90.0, 10.0], &shot, &registry);
        let read = actors(&kept).unwrap().remove(0);
        assert_eq!((read.identifier.as_str(), read.position, read.yaw, shot_record(&kept)), ("minecraft:thrown_trident", [0.5, 70.0, 0.5], 90.0, Some(shot)));
    }
}
