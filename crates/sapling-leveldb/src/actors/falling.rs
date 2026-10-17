//! The actor a falling block is kept as (BDS `FallingBlockActor::{add,read}AdditionalSaveData`).

use acacia_proto::nbt::{self, LittleEndian, Value};
use acacia_world::BlockRegistry;

use super::{floats, written};
use crate::states::{block_of_hash, network_hash};

const BLOCK: &str = "FallingBlock";
const TIME: &str = "Time";

/// A new falling block actor of the block whose network hash is `hash`, with the bottom of its box
/// at `position`, `time` ticks into its fall; `None` for a block the registry does not name.
// TODO: worlds before `FallingBlock` kept `Tile` and `Data`, which BDS still reads; this does not.
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

/// The network hash of the block the falling block kept as `compound` is, and how long it has
/// fallen, if it says.
pub fn falling_block(compound: &[u8]) -> Option<(u32, u8)> {
    let actor = nbt::read::<LittleEndian>(&mut &compound[..]).ok()?.value;
    let time = match actor.get(TIME) {
        Some(Value::Byte(time)) => *time as u8,
        _ => 0,
    };
    Some((network_hash(actor.get(BLOCK)?)?, time))
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
}
