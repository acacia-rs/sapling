//! A catalyst's cursors kept in its block actor, as BDS's `SculkCatalystBlockActor` saves them
//! (148b48370) and loads them (148b48710): `cursors`, a list of `{x, y, z: int, charge, update,
//! decay, facing: short}`, written while any are left; at most 32 read, into a catalyst holding none.

use acacia_proto::nbt::{List, Value};
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::{ChunkPos, new_block_entity};
use sapling_worldgen::sculk::Cursor;

use super::catalysts;
use crate::components::Layer;

const ENTITY: &str = "SculkCatalyst";
const CURSORS: &str = "cursors";
const MOST: usize = 32;
/// `TAG_Compound`, the cursors' element tag.
const COMPOUND: u8 = 10;

fn to_value(cursor: &Cursor) -> Value {
    let [x, y, z] = cursor.pos;
    let short = |value: i32| Value::Short(value as i16);
    let fields = [
        ("x", Value::Int(x)),
        ("y", Value::Int(y)),
        ("z", Value::Int(z)),
        ("charge", short(cursor.charge)),
        ("update", short(cursor.update_delay)),
        ("decay", short(cursor.decay_delay)),
        ("facing", short(cursor.facings)),
    ];
    Value::Compound(fields.into_iter().map(|(key, value)| (key.into(), value)).collect())
}

/// A saved cursor; a missing field keeps BDS's default (a fresh cursor's).
fn from_value(value: &Value) -> Cursor {
    let number = |key: &str, default: i32| match value.get(key) {
        Some(Value::Int(v)) => *v,
        Some(Value::Short(v)) => (*v).into(),
        _ => default,
    };
    let fresh = Cursor::new([0; 3], 0);
    Cursor {
        pos: [number("x", 0), number("y", 0), number("z", 0)],
        charge: number("charge", fresh.charge),
        update_delay: number("update", fresh.update_delay),
        decay_delay: number("decay", fresh.decay_delay),
        facings: number("facing", fresh.facings),
    }
}

fn is_catalyst(nbt: &Value) -> bool {
    matches!(nbt.get("id"), Some(Value::String(id)) if id.as_str() == ENTITY)
}

/// Writes `cursors` into the block actor of the catalyst at `pos`, where that changes it.
pub(super) fn keep(world: &mut World, layer: Entity, pos: BlockPos, cursors: &[Cursor]) {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let before = level.block_entity(pos).map(|entity| entity.nbt.clone()).filter(is_catalyst);
    if before.is_none() && cursors.is_empty() {
        return;
    }
    let mut entity = before.clone().unwrap_or_else(|| new_block_entity(ENTITY, pos));
    let Value::Compound(fields) = &mut entity else { return };
    fields.retain(|(key, _)| key != CURSORS);
    if !cursors.is_empty() {
        fields.push((CURSORS.into(), Value::List(List { tag: COMPOUND, items: cursors.iter().map(to_value).collect() })));
    }
    if before.as_ref() != Some(&entity) {
        world.get_mut::<Layer>(layer).map(|mut level| level.world.set_block_entity(pos, &entity));
    }
}

/// Takes up the saved cursors of the catalysts of a column that came in.
pub(crate) fn arrived(world: &mut World, layer: Entity, home: ChunkPos) {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let saved = |nbt: &Value| match nbt.get(CURSORS) {
        Some(Value::List(list)) => Some(list.items.iter().filter(|item| matches!(item, Value::Compound(_))).take(MOST).map(from_value).collect::<Vec<_>>()),
        _ => None,
    };
    let found: Vec<(BlockPos, Vec<Cursor>)> = level.block_entities_in(home).into_iter().filter(|entity| is_catalyst(&entity.nbt)).filter_map(|entity| Some((entity.pos, saved(&entity.nbt)?))).collect();
    if found.is_empty() {
        return;
    }
    let mut catalysts = catalysts(world);
    for (pos, cursors) in found {
        let held = catalysts.cursors.entry((layer, pos)).or_default();
        if held.is_empty() {
            *held = cursors;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cursor_comes_back_as_it_was_saved() {
        let cursor = Cursor { pos: [-3, 64, 9], charge: 312, update_delay: 2, decay_delay: 0, facings: -1 };
        let back = from_value(&to_value(&cursor));
        assert_eq!((back.pos, back.charge, back.update_delay, back.decay_delay, back.facings), (cursor.pos, 312, 2, 0, -1));
        let bare = from_value(&Value::Compound(vec![("x".into(), Value::Int(1))]));
        assert_eq!((bare.pos, bare.update_delay, bare.decay_delay, bare.facings), ([1, 0, 0], 0, 1, -1));
    }
}
