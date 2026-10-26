//! A spawner block's entity as a list of fields, read and written by name.

use acacia_proto::nbt::{List, Str, Value};
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use crate::components::Layer;

/// `TAG_Compound`, the element tag of a list of compounds.
const COMPOUND: u8 = 10;
const LONG: u8 = 4;

pub(super) struct Fields(pub Vec<(Str, Value)>);

impl Fields {
    /// What the block at `pos` holds; `None` where it holds nothing.
    pub fn at(world: &World, layer: Entity, pos: BlockPos) -> Option<Self> {
        match world.get::<Layer>(layer)?.world.block_entity(pos)?.nbt.clone() {
            Value::Compound(fields) => Some(Self(fields)),
            _ => None,
        }
    }

    /// The compound under `key`; empty where there is none.
    pub fn inner(&self, key: &str) -> Self {
        match self.get(key) {
            Some(Value::Compound(fields)) => Self(fields.clone()),
            _ => Self(Vec::new()),
        }
    }

    /// The list under `key`; empty where there is none.
    pub fn list(&self, key: &str) -> Vec<Value> {
        match self.get(key) {
            Some(Value::List(list)) => list.items.clone(),
            _ => Vec::new(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.iter().find(|(name, _)| name == key).map(|(_, value)| value)
    }

    pub fn set(&mut self, key: &str, value: Value) {
        match self.0.iter_mut().find(|(name, _)| name == key) {
            Some((_, old)) => *old = value,
            None => self.0.push((key.into(), value)),
        }
    }

    /// A whole number of any width; `or` where it is missing.
    pub fn int(&self, key: &str, or: i64) -> i64 {
        match self.get(key) {
            Some(Value::Byte(v)) => (*v).into(),
            Some(Value::Short(v)) => (*v).into(),
            Some(Value::Int(v)) => (*v).into(),
            Some(Value::Long(v)) => *v,
            _ => or,
        }
    }

    pub fn float(&self, key: &str, or: f32) -> f32 {
        match self.get(key) {
            Some(Value::Float(v)) => *v,
            Some(Value::Double(v)) => *v as f32,
            _ => or,
        }
    }

    pub fn string(&self, key: &str) -> Option<&str> {
        match self.get(key) {
            Some(Value::String(text)) => Some(text),
            _ => None,
        }
    }

    pub fn into_value(self) -> Value {
        Value::Compound(self.0)
    }
}

/// A list of compounds, empty or not, as BDS writes one.
pub(super) fn compounds(items: Vec<Value>) -> Value {
    Value::List(List { tag: COMPOUND, items })
}

/// A list of longs, as BDS writes `rewarded_players`.
pub(super) fn longs(items: &[i64]) -> Value {
    Value::List(List { tag: LONG, items: items.iter().map(|id| Value::Long(*id)).collect() })
}

/// The longs of a list; what is not one is left out.
pub(super) fn as_longs(items: &[Value]) -> Vec<i64> {
    items.iter().filter_map(|item| if let Value::Long(id) = item { Some(*id) } else { None }).collect()
}
