//! The actor a falling block is kept as (BDS `FallingBlockActor::{add,read}AdditionalSaveData`).
//! Read as BDS reads it (145985b20): a record with both `Tile` and `Data`, as worlds before
//! `FallingBlock` kept, is the numeric block `Tile` of meta `Data` (bytes, 0 if of another type).

use acacia_proto::nbt::{self, LittleEndian, Value};
use acacia_world::BlockRegistry;

use super::{floats, with_fields, written};
use crate::Error;
use crate::states::{block_of_hash, compound_of, network_hash};
use crate::upgrade;

const BLOCK: &str = "FallingBlock";
const TIME: &str = "Time";
const TILE: &str = "Tile";
const DATA: &str = "Data";

/// A new falling block actor of the block whose network hash is `hash`, with the bottom of its box
/// at `position`, `time` ticks into its fall; `None` for a block the registry does not name.
pub fn new_falling_block_actor(unique_id: i64, hash: u32, registry: &BlockRegistry, position: [f32; 3], motion: [f32; 3], time: u8) -> Option<Vec<u8>> {
    let fields = [
        ("UniqueID", Value::Long(unique_id)),
        ("identifier", Value::String("minecraft:falling_block".into())),
        ("Pos", floats(&position)),
        ("Rotation", floats(&[0.0, 0.0])),
        ("Motion", floats(&motion)),
        (BLOCK, block_of_hash(registry, hash)?),
        (TIME, Value::Byte(time as i8)),
    ];
    Some(written(Value::Compound(fields.into_iter().map(|(key, value)| (key.into(), value)).collect())))
}

/// `compound`, a falling block's, `time` ticks into its fall: its low byte, as BDS writes `mTime`
/// (`addAdditionalSaveData` 145985a00: `movzx` of the int's first byte, `putByte`).
pub fn timed_falling_block(compound: &[u8], time: u32) -> Result<Vec<u8>, Error> {
    with_fields(compound, [(TIME, Value::Byte(time as u8 as i8))])
}

/// The network hash of the block the falling block kept as `compound` is, and how long it has
/// fallen, if it says.
pub fn falling_block(compound: &[u8]) -> Option<(u32, u8)> {
    let actor = nbt::read::<LittleEndian>(&mut &compound[..]).ok()?.value;
    let byte = |key: &str| match actor.get(key) {
        Some(Value::Byte(value)) => *value as u8,
        _ => 0,
    };
    let block = match (actor.get(TILE), actor.get(DATA)) {
        (Some(_), Some(_)) => {
            let (name, states) = upgrade::from_legacy(upgrade::legacy_name(byte(TILE).into())?, byte(DATA).into())?;
            network_hash(&Value::Compound(vec![("name".into(), Value::String(name.as_str().into())), ("states".into(), compound_of(&states))]))?
        }
        _ => network_hash(actor.get(BLOCK)?)?,
    };
    Some((block, byte(TIME)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actors::actors;

    #[test]
    fn a_falling_block_is_read_back_as_it_was_written() {
        let registry = BlockRegistry::vanilla();
        let sand = (0..registry.len() as u32).filter_map(|id| registry.get(id)).find(|state| state.name == "minecraft:sand").unwrap().network_hash;
        let kept = new_falling_block_actor(5, sand, &registry, [0.5, 70.0, 0.5], [0.0, -0.4, 0.0], 12).unwrap();
        let read = actors(&kept).unwrap().remove(0);
        assert_eq!((read.identifier.as_str(), read.position, falling_block(&kept)), ("minecraft:falling_block", [0.5, 70.0, 0.5], Some((sand, 12))));
    }

    #[test]
    fn a_fall_past_255_ticks_is_kept_as_its_low_byte() {
        let registry = BlockRegistry::vanilla();
        let sand = (0..registry.len() as u32).filter_map(|id| registry.get(id)).find(|state| state.name == "minecraft:sand").unwrap().network_hash;
        let kept = new_falling_block_actor(5, sand, &registry, [0.5, 70.0, 0.5], [0.0; 3], 1).unwrap();
        let times = [200, 255, 256, 300].map(|time| falling_block(&timed_falling_block(&kept, time).unwrap()).map(|(_, time)| time));
        assert_eq!(times, [Some(200), Some(255), Some(0), Some(44)]);
    }

    #[test]
    fn an_old_worlds_falling_block_is_read_by_its_tile_and_data() {
        let registry = BlockRegistry::vanilla();
        let name = |hash: u32| registry.runtime_id_from_hash(hash).and_then(|id| registry.get(id)).map(|state| state.name.to_string());
        let old = |tile: i8, data: i8| written(Value::Compound(vec![("Tile".into(), Value::Byte(tile)), ("Data".into(), Value::Byte(data)), ("Time".into(), Value::Byte(3))]));
        let sand = (0..registry.len() as u32).filter_map(|id| registry.get(id)).find(|state| state.name == "minecraft:sand").unwrap().network_hash;
        assert_eq!(falling_block(&old(12, 0)), Some((sand, 3)));
        assert_eq!(falling_block(&old(145u8 as i8, 4)).and_then(|(block, _)| name(block)), Some("minecraft:chipped_anvil".to_owned()));
    }
}
