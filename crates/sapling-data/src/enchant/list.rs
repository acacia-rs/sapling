//! A stack's `ench` list read whole and written back: `{id: short, lvl: short}` compounds in the
//! order they were added.

use acacia_proto::nbt::{List, Value};

use crate::gear::with_field;

const ENCH: &str = "ench";
const COMPOUND: u8 = 10;

/// Every enchantment on a stack whose NBT is `tag`, by id, with its level; in its order.
pub fn all(tag: Option<&Value>) -> Vec<(i16, u8)> {
    let Some(Value::List(list)) = tag.and_then(|tag| tag.get(ENCH)) else { return Vec::new() };
    let entry = |entry: &Value| match (entry.get("id"), entry.get("lvl")) {
        (Some(Value::Short(id)), Some(Value::Short(level))) if *level > 0 => Some((*id, (*level).min(i16::from(u8::MAX)) as u8)),
        _ => None,
    };
    list.items.iter().filter_map(entry).collect()
}

/// `tag` with these enchantments as its `ench` list; with none, no list.
pub fn with_all(tag: Option<Value>, levels: &[(i16, u8)]) -> Option<Value> {
    let entry = |(id, level): &(i16, u8)| Value::Compound(vec![("id".into(), Value::Short(*id)), ("lvl".into(), Value::Short((*level).into()))]);
    let list = (!levels.is_empty()).then(|| Value::List(List { tag: COMPOUND, items: levels.iter().map(entry).collect() }));
    with_field(tag, ENCH, list)
}
