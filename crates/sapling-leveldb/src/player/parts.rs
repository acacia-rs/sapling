//! The pieces of a player record: item lists, effects, attributes, and setting a field of a
//! compound that may hold others.

use acacia_proto::nbt::{List, Str, Value};
use acacia_world::BlockRegistry;

use crate::{Error, ItemStack};

pub(super) const COMPOUND: u8 = 10;
pub(super) const FLOAT: u8 = 5;

/// Replaces the field `key`, or adds it after the others.
pub(super) fn set(fields: &mut Vec<(Str, Value)>, key: &str, value: Value) {
    match fields.iter_mut().find(|(name, _)| name == key) {
        Some((_, old)) => *old = value,
        None => fields.push((key.into(), value)),
    }
}

pub(super) fn remove(fields: &mut Vec<(Str, Value)>, key: &str) {
    fields.retain(|(name, _)| name != key);
}

pub(super) fn floats(values: &[f32]) -> Value {
    Value::List(List { tag: FLOAT, items: values.iter().copied().map(Value::Float).collect() })
}

/// Every slot in order, an empty one as a stack without a name.
pub(crate) fn stacks(slots: &[Option<ItemStack>], registry: &BlockRegistry) -> Value {
    let stack = |slot: &Option<ItemStack>| Value::Compound(slot.as_ref().map_or_else(ItemStack::no_fields, |stack| stack.fields(registry)));
    Value::List(List { tag: COMPOUND, items: slots.iter().map(stack).collect() })
}

/// Stacks each with the `Slot` it lies in; empty slots are left out.
pub(super) fn slotted(slots: &[Option<ItemStack>], registry: &BlockRegistry) -> Value {
    let held = |(slot, stack): (usize, &Option<ItemStack>)| {
        let mut fields = stack.as_ref()?.fields(registry);
        fields.push(("Slot".into(), Value::Byte(slot as i8)));
        Some(Value::Compound(fields))
    };
    Value::List(List { tag: COMPOUND, items: slots.iter().enumerate().filter_map(held).collect() })
}

pub(super) fn read_slotted(stored: &[Value]) -> Result<Vec<Option<ItemStack>>, Error> {
    let mut slots = Vec::new();
    for stored in stored {
        let (Some(Value::Byte(slot)), Some(stack)) = (stored.get("Slot"), ItemStack::parse(stored)) else { continue };
        let slot = usize::try_from(*slot).map_err(|_| Error::Player("a negative slot"))?;
        slots.resize(slots.len().max(slot + 1), None);
        slots[slot] = Some(stack);
    }
    Ok(slots)
}

/// BDS's `MobEffectInstance` fields (BDSRE save 141c57070): `Id` and `Amplifier` bytes, the
/// durations ints, `Ambient` and `ShowParticles` bytes.
const EFFECT: [&str; 4] = ["Id", "Amplifier", "Duration", "ShowParticles"];
const DURATIONS: [&str; 3] = ["DurationEasy", "DurationNormal", "DurationHard"];

pub(crate) fn effect(kept: &Value) -> Option<[i32; 4]> {
    let part = |key: &str| match kept.get(key) {
        Some(Value::Byte(part)) => Some(i32::from(*part)),
        Some(Value::Int(part)) => Some(*part),
        _ => None,
    };
    Some([part(EFFECT[0])?, part(EFFECT[1])?, part(EFFECT[2])?, part(EFFECT[3]).unwrap_or(1)])
}

pub(super) fn effect_fields([id, amplifier, left, shown]: &[i32; 4]) -> Value {
    let mut fields = vec![(EFFECT[0].into(), Value::Byte(*id as i8)), (EFFECT[1].into(), Value::Byte(*amplifier as i8)), (EFFECT[2].into(), Value::Int(*left))];
    fields.extend(DURATIONS.map(|key| (key.into(), Value::Int(*left))));
    fields.extend([("Ambient".into(), Value::Byte(0)), (EFFECT[3].into(), Value::Byte(*shown as i8))]);
    Value::Compound(fields)
}

/// Sets each named attribute's `Current` in `stored` (a record's `Attributes`), keeping what else
/// it holds; one it lacks is added with only its `Name` and `Current`.
pub(super) fn with_attributes(stored: Option<&Value>, currents: impl IntoIterator<Item = (&'static str, f32)>) -> Value {
    let mut items = match stored {
        Some(Value::List(list)) if list.tag == COMPOUND => list.items.clone(),
        _ => Vec::new(),
    };
    for (name, current) in currents {
        let named = |item: &&mut Value| matches!(item.get("Name"), Some(Value::String(kept)) if kept == name);
        match items.iter_mut().find(named) {
            Some(Value::Compound(fields)) => set(fields, "Current", Value::Float(current)),
            _ => items.push(Value::Compound(vec![("Name".into(), Value::String(name.into())), ("Current".into(), Value::Float(current))])),
        }
    }
    Value::List(List { tag: COMPOUND, items })
}
