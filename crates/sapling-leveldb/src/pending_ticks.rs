//! A column's `PendingTicks` record: the block ticks that were waiting when it was saved, as a
//! BDS 1.26.5x save holds them (research/captures/bds-pendingticks.txt, read with the `records`
//! example): one compound with `currentTick` (int) and the list `tickList`, each entry the
//! `blockState` it is for (`{name, states, version}`), the tick it is due (`time`, a long on the
//! same count as `currentTick`) and its cell (`x`, `y`, `z`). A cell queued for two ticks is
//! listed twice; liquids name their own state, observers their block's first.

use acacia_proto::nbt::{self, List, LittleEndian, Nbt, Value};
use acacia_world::BlockRegistry;
use bytes::BytesMut;

use crate::Error;
use crate::states::{block_of_hash, palette_block};

const COMPOUND: u8 = 10;

/// One waiting tick: the block it is for (its name and states), its cell and when it is due.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingTick {
    pub name: String,
    /// As the registry spells them (`key=value`, sorted).
    pub properties: String,
    pub pos: [i32; 3],
    pub time: i64,
}

/// The tick the record was saved at and what waited; nothing for an empty record.
pub fn decode_pending_ticks(record: &[u8]) -> Result<(i64, Vec<PendingTick>), Error> {
    if record.is_empty() {
        return Ok((0, Vec::new()));
    }
    let root = nbt::read::<LittleEndian>(&mut &record[..])?.value;
    let number = |kept: &Value, key: &str| match kept.get(key) {
        Some(Value::Int(value)) => Some(i64::from(*value)),
        Some(Value::Long(value)) => Some(*value),
        _ => None,
    };
    let entry = |kept: &Value| {
        let (name, properties) = palette_block(kept.get("blockState")?)?;
        let pos = [number(kept, "x")? as i32, number(kept, "y")? as i32, number(kept, "z")? as i32];
        Some(PendingTick { name, properties, pos, time: number(kept, "time")? })
    };
    let ticks = match root.get("tickList") {
        Some(Value::List(list)) => list.items.iter().filter_map(entry).collect(),
        _ => Vec::new(),
    };
    Ok((number(&root, "currentTick").unwrap_or(0), ticks))
}

/// The record of `ticks` saved at `current`, each for the state `(runtime id, tick)` names;
/// empty when nothing waits, for the record to be deleted.
pub fn encode_pending_ticks(registry: &BlockRegistry, current: i64, ticks: &[([i32; 3], u32, i64)]) -> Vec<u8> {
    let entry = |&([x, y, z], id, time): &([i32; 3], u32, i64)| {
        let state = block_of_hash(registry, registry.network_hash(id))?;
        Some(Value::Compound(vec![
            ("blockState".into(), state),
            ("time".into(), Value::Long(time)),
            ("x".into(), Value::Int(x)),
            ("y".into(), Value::Int(y)),
            ("z".into(), Value::Int(z)),
        ]))
    };
    let items: Vec<Value> = ticks.iter().filter_map(entry).collect();
    if items.is_empty() {
        return Vec::new();
    }
    let root = Value::Compound(vec![("currentTick".into(), Value::Int(current as i32)), ("tickList".into(), Value::List(List { tag: COMPOUND, items }))]);
    let mut out = BytesMut::new();
    nbt::write::<LittleEndian>(&mut out, &Nbt { name: String::new(), value: root });
    out.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_read_back_as_they_were_written() {
        let registry = BlockRegistry::vanilla();
        let water = registry.find("minecraft:water", "liquid_depth=2").unwrap();
        let stone = registry.find("minecraft:stone", "").unwrap();
        let record = encode_pending_ticks(registry, 375_266, &[([3, -20, 12], water, 375_270), ([4, 70, -9], stone, 375_300)]);
        let (current, ticks) = decode_pending_ticks(&record).unwrap();
        assert_eq!(current, 375_266);
        let read: Vec<_> = ticks.iter().map(|tick| (tick.name.as_str(), tick.properties.as_str(), tick.pos, tick.time)).collect();
        assert_eq!(read, [("minecraft:water", "liquid_depth=2", [3, -20, 12], 375_270), ("minecraft:stone", "", [4, 70, -9], 375_300)]);
        assert!(encode_pending_ticks(registry, 5, &[]).is_empty());
        assert_eq!(decode_pending_ticks(&[]).unwrap(), (0, Vec::new()));
    }
}
