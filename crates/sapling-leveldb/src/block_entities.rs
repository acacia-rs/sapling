//! The BlockEntities record (records.rs): each compound names its block in `x`, `y`, `z`.

use acacia_proto::nbt::{self, List, LittleEndian, Nbt, Network, Str, Value};
use bytes::{Bytes, BytesMut};

use crate::Error;
use crate::container::without_items;
use crate::records::compounds;

/// What a block holds beyond its state: a sign's text, a chest's items.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockEntity {
    pub pos: [i32; 3],
    pub nbt: Value,
}

/// Every block entity BDS sends carries it, 0 (each id in research/captures/bds-*.txt).
const VERSION: &str = "BlockEntityVersion";
/// Kinds whose `Items` BDS shows, an empty list as one of no tag (research/captures/bds-s7ents.txt);
/// a shulker box holding stacks was not drilled.
const ITEMS_SHOWN: [&str; 2] = ["Cauldron", "ShulkerBox"];
/// Signs are shown with each side's filtered text and who edits them, none: fields of the wire alone.
const SIGNS: [&str; 2] = ["Sign", "HangingSign"];
const SIDES: [&str; 2] = ["FrontText", "BackText"];
const FILTERED: &str = "FilteredText";
const LOCKED: &str = "LockedForEditingBy";

impl BlockEntity {
    /// What clients are sent of it, in a chunk or a `BlockEntityData` packet: as BDS's, with
    /// [`VERSION`] and every compound's keys in order.
    pub fn shown(&self) -> Value {
        let id = match self.nbt.get("id") {
            Some(Value::String(id)) => id.to_string(),
            _ => String::new(),
        };
        let mut shown = if ITEMS_SHOWN.contains(&id.as_str()) { self.nbt.clone() } else { without_items(&self.nbt) };
        if let Value::Compound(fields) = &mut shown {
            if !fields.iter().any(|(key, _)| key == VERSION) {
                fields.push((VERSION.into(), Value::Int(0)));
            }
            for (key, value) in fields.iter_mut() {
                match value {
                    Value::List(list) if key == "Items" && list.items.is_empty() => list.tag = 0,
                    Value::Compound(side) if SIGNS.contains(&id.as_str()) && SIDES.contains(&key.as_ref()) && !side.iter().any(|(key, _)| key == FILTERED) => {
                        side.push((FILTERED.into(), Value::String(Default::default())));
                    }
                    _ => {}
                }
            }
            if SIGNS.contains(&id.as_str()) && !fields.iter().any(|(key, _)| key == LOCKED) {
                fields.push((LOCKED.into(), Value::Long(-1)));
            }
        }
        sorted(shown)
    }

    /// What a `BlockEntityData` packet carries of it: [`BlockEntity::shown`], but a trial
    /// spawner's, a vault's and a crafter's, which BDS writes as their own, without even an id
    /// (research/captures/bds-s7ents.txt; one that held anything was not drilled).
    pub fn updated(&self) -> Value {
        let field = |key: &str, value| (Str::from(key), value);
        let kept = |key: &str| self.nbt.get(key).cloned();
        match self.nbt.get("id") {
            Some(Value::String(id)) if id == "TrialSpawner" => Value::Compound(Vec::new()),
            Some(Value::String(id)) if id == "Vault" => {
                let range = self.nbt.get("config").and_then(|config| config.get("deactivation_range")).cloned().unwrap_or(Value::Float(4.5));
                let nothing = ["Count", "Damage", "Name", "WasPickedUp"].into_iter().zip([Value::Byte(0), Value::Short(0), Value::String(Str::default()), Value::Byte(0)]);
                let shown = Value::Compound(nothing.map(|(key, value)| field(key, value)).collect());
                Value::Compound(vec![field("connected_particle_range", range), field("connected_players", Value::List(List { tag: 0, items: Vec::new() })), field("display_item", shown)])
            }
            Some(Value::String(id)) if id == "Crafter" => Value::Compound(vec![
                field("crafting_ticks_remaining", kept("crafting_ticks_remaining").unwrap_or(Value::Int(0))),
                field("disabled_slots", kept("disabled_slots").unwrap_or(Value::Short(0))),
            ]),
            _ => self.shown(),
        }
    }

    /// [`BlockEntity::shown`] as a chunk carries it.
    pub fn network(&self) -> Bytes {
        let mut out = BytesMut::new();
        nbt::write::<Network>(&mut out, &Nbt { name: String::new(), value: self.shown() });
        out.freeze()
    }
}

/// `value` with the keys of each compound in it sorted, as BDS's compounds (maps) keep them.
fn sorted(value: Value) -> Value {
    match value {
        Value::Compound(fields) => {
            let mut fields: Vec<_> = fields.into_iter().map(|(key, value)| (key, sorted(value))).collect();
            fields.sort_by(|a, b| a.0.cmp(&b.0));
            Value::Compound(fields)
        }
        Value::List(list) => Value::List(List { tag: list.tag, items: list.items.into_iter().map(sorted).collect() }),
        other => other,
    }
}

/// The record with `entity` as the one block entity of block `pos`.
pub fn with_block_entity(record: &[u8], pos: [i32; 3], entity: &Value) -> Result<Vec<u8>, Error> {
    let mut out = BytesMut::from(&without_block_entity(record, pos)?[..]);
    nbt::write::<LittleEndian>(&mut out, &Nbt { name: String::new(), value: entity.clone() });
    Ok(out.to_vec())
}

fn position(entity: &Value) -> [Option<i32>; 3] {
    ["x", "y", "z"].map(|key| match entity.get(key) {
        Some(Value::Int(at)) => Some(*at),
        _ => None,
    })
}

/// The block entities of a record; compounds that name no block are left out.
pub fn block_entities(record: &[u8]) -> Result<Vec<BlockEntity>, Error> {
    let placed = |(_, nbt): (&[u8], Value)| match position(&nbt) {
        [Some(x), Some(y), Some(z)] => Some(BlockEntity { pos: [x, y, z], nbt }),
        _ => None,
    };
    Ok(compounds(record)?.into_iter().filter_map(placed).collect())
}

/// The record without the compounds of block `pos`; the others keep their bytes.
pub fn without_block_entity(record: &[u8], pos: [i32; 3]) -> Result<Vec<u8>, Error> {
    let elsewhere = |entity: &Value| position(entity) != pos.map(Some);
    Ok(compounds(record)?.into_iter().filter(|(_, entity)| elsewhere(entity)).flat_map(|(raw, _)| raw.iter().copied()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::records::compound;

    fn entity(id: &str, [x, y, z]: [i32; 3]) -> Vec<u8> {
        compound(vec![("id", Value::String(id.into())), ("x", Value::Int(x)), ("y", Value::Int(y)), ("z", Value::Int(z))])
    }

    #[test]
    fn only_the_compounds_of_the_block_leave() {
        let (chest, sign) = (entity("Chest", [1, 64, -3]), entity("Sign", [1, 65, -3]));
        let record = [&chest[..], &sign, &chest].concat();
        assert_eq!(without_block_entity(&record, [1, 64, -3]).unwrap(), sign);
        assert_eq!(without_block_entity(&record, [1, 65, -3]).unwrap(), [&chest[..], &chest].concat());
        assert_eq!(without_block_entity(&record, [0, 0, 0]).unwrap(), record);
        assert_eq!(without_block_entity(&[], [0, 0, 0]).unwrap(), Vec::<u8>::new());
        assert!(without_block_entity(&record[..record.len() - 2], [0, 0, 0]).is_err());
    }

    #[test]
    fn a_record_reads_as_block_entities_a_client_can_read() {
        let nameless = compound(vec![("id", Value::String("Chest".into()))]);
        let record = [&entity("Sign", [1, 65, -3])[..], &nameless, &entity("Chest", [2, 4, 0])].concat();
        let read = block_entities(&record).unwrap();
        assert_eq!(read.iter().map(|entity| entity.pos).collect::<Vec<_>>(), [[1, 65, -3], [2, 4, 0]]);
        let wire = read[0].network();
        let back = nbt::read::<Network>(&mut &wire[..]).unwrap();
        assert_eq!(back.value.get("id"), Some(&Value::String("Sign".into())));
    }

    #[test]
    fn a_block_entity_is_shown_as_bds_shows_it_versioned_and_sorted() {
        let plant = Value::Compound(vec![("version".into(), Value::Int(1)), ("name".into(), Value::String("minecraft:poppy".into()))]);
        let mut pot = crate::new_block_entity("FlowerPot", [1, -26, -6]);
        if let Value::Compound(fields) = &mut pot {
            fields.push(("PlantBlock".into(), plant));
        }
        let shown = BlockEntity { pos: [1, -26, -6], nbt: pot }.shown();
        let Value::Compound(fields) = &shown else { panic!("{shown:?}") };
        assert_eq!(fields.iter().map(|(key, _)| key.as_str()).collect::<Vec<_>>(), ["BlockEntityVersion", "PlantBlock", "id", "x", "y", "z"]);
        let Some(Value::Compound(plant)) = shown.get("PlantBlock") else { panic!("{shown:?}") };
        assert_eq!(plant.iter().map(|(key, _)| key.as_str()).collect::<Vec<_>>(), ["name", "version"]);
    }

    #[test]
    fn a_cauldron_shows_its_empty_items_and_a_sign_its_wire_fields() {
        let with = |id: &str, more: Vec<(&str, Value)>| {
            let mut entity = crate::new_block_entity(id, [0, 0, 0]);
            if let Value::Compound(fields) = &mut entity {
                fields.extend(more.into_iter().map(|(key, value)| (key.into(), value)));
            }
            BlockEntity { pos: [0, 0, 0], nbt: entity }.shown()
        };
        let cauldron = with("Cauldron", vec![("Items", Value::List(List { tag: 10, items: Vec::new() }))]);
        assert_eq!(cauldron.get("Items"), Some(&Value::List(List { tag: 0, items: Vec::new() })));
        let chest = with("Chest", vec![("Items", Value::List(List { tag: 10, items: Vec::new() }))]);
        assert_eq!(chest.get("Items"), None);
        let side = || Value::Compound(vec![("Text".into(), Value::String("".into()))]);
        let sign = with("Sign", vec![("FrontText", side()), ("BackText", side())]);
        assert_eq!(sign.get("LockedForEditingBy"), Some(&Value::Long(-1)));
        for side in ["FrontText", "BackText"] {
            assert_eq!(sign.get(side).and_then(|side| side.get("FilteredText")), Some(&Value::String("".into())));
        }
    }
}
