//! A block entity's compound read and changed field by field, as command and structure blocks
//! keep theirs.

use acacia_proto::nbt::{Str, Value};
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::{keep_entity, level};
use crate::components::Layer;

pub(super) struct Fields(pub(super) Vec<(Str, Value)>);

impl Fields {
    /// The fields of `entity` where its `id` is `id`.
    pub(super) fn of(entity: &Value, id: &str) -> Option<Fields> {
        match entity {
            Value::Compound(fields) if matches!(entity.get("id"), Some(Value::String(kept)) if &**kept == id) => Some(Fields(fields.clone())),
            _ => None,
        }
    }

    pub(super) fn read(world: &World, layer: Entity, pos: BlockPos, id: &str) -> Option<Fields> {
        Fields::of(&level(world, layer)?.block_entity(pos)?.nbt, id)
    }

    pub(super) fn get(&self, key: &str) -> Option<&Value> {
        self.0.iter().find(|(name, _)| &**name == key).map(|(_, value)| value)
    }

    pub(super) fn int(&self, key: &str) -> i64 {
        match self.get(key) {
            Some(Value::Byte(n)) => (*n).into(),
            Some(Value::Int(n)) => (*n).into(),
            Some(Value::Long(n)) => *n,
            _ => 0,
        }
    }

    pub(super) fn float(&self, key: &str) -> f32 {
        match self.get(key) {
            Some(Value::Float(f)) => *f,
            _ => self.int(key) as f32,
        }
    }

    pub(super) fn flag(&self, key: &str) -> bool {
        self.int(key) != 0
    }

    pub(super) fn text(&self, key: &str) -> String {
        match self.get(key) {
            Some(Value::String(text)) => text.to_string(),
            _ => String::new(),
        }
    }

    pub(super) fn put(&mut self, key: &str, value: Value) {
        match self.0.iter_mut().find(|(name, _)| &**name == key) {
            Some((_, kept)) => *kept = value,
            None => self.0.push((key.into(), value)),
        }
    }

    pub(super) fn set_flag(&mut self, key: &str, on: bool) {
        self.put(key, Value::Byte(on.into()));
    }

    pub(super) fn set_text(&mut self, key: &str, text: &str) {
        self.put(key, Value::String(text.into()));
    }

    /// Keeps the entity, sorted as BDS sends it, shown to those who see the block.
    pub(super) fn show(mut self, world: &mut World, layer: Entity, pos: BlockPos) {
        self.0.sort_by(|a, b| a.0.cmp(&b.0));
        keep_entity(world, layer, pos, &Value::Compound(self.0));
    }

    /// Keeps the entity without showing it.
    pub(super) fn keep(mut self, world: &mut World, layer: Entity, pos: BlockPos) {
        self.0.sort_by(|a, b| a.0.cmp(&b.0));
        if let Some(mut level) = world.get_mut::<Layer>(layer) {
            level.world.set_block_entity(pos, &Value::Compound(self.0));
        }
    }
}
