//! Brushing suspicious sand and gravel as BDS 1.26 does (docs/research/structure-block-entities.md
//! "Brushing"; research/captures/bds-brush.txt). A held brush arrives as a `ClickBlock` with the
//! simulation tick as its trigger, every tick; each one brushes. The block's entity counts strokes
//! (`brush_count`) at most one per [`STROKE`] ticks, and `brushed_progress` follows the count; the
//! tenth leaves the rolled item beside the face first brushed. Left alone, the count falls back.

use std::collections::HashMap;

use acacia_proto::nbt::Value;
use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::{TransactionUseItem, TransactionUseItemTriggerType, Vec3f};
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};
use sapling_world::{ItemStack, new_block_entity};

use super::{keep_entity, level, send};
use crate::components::{InLayer, Tick};
use crate::inventory::Inventory;

const BRUSH: &str = "minecraft:brush";
const LASTS: u16 = 64;
const ENTITY: &str = "BrushableBlock";
const COUNT: &str = "brush_count";
const DIRECTION: &str = "brush_direction";
const ITEM: &str = "item";
/// `brush_direction` before the first stroke.
const NO_FACE: i8 = 6;
/// Ticks a counted stroke holds off the next (BDS 141f23fa0: the next is due 10 after the last was due).
const STROKE: u64 = 10;
const STROKES: i32 = 10;
/// A stroke's check of the count falls this long after it; until then rechecks wait [`RECHECK`].
const RESET_AFTER: u64 = 40;
const RECHECK: u64 = 20;
/// Once falling, the count loses 2 every [`FALL_EVERY`] ticks (BDS 141f25300).
const FALL_EVERY: u64 = 10;
/// Ticks between a player's `brush` sounds, heard on the tick after (BDS 147a943b0).
const SOUND_EVERY: u64 = 8;

/// What BDS keeps of a block being brushed but never saves.
#[derive(Default)]
struct Strokes {
    next_due: Option<u64>,
    reset_at: u64,
    resetting: bool,
}

#[derive(Resource, Default)]
pub(crate) struct Brushing {
    blocks: HashMap<(Entity, BlockPos), Strokes>,
    /// Queued checks of the count, one per counted stroke and never merged, as BDS queues them.
    checks: Vec<(u64, Entity, BlockPos)>,
    /// When each player's next `brush` sound may play.
    sounds: HashMap<Entity, u64>,
}

/// The block a brushed one becomes.
fn uncovered(name: &str) -> Option<&'static str> {
    match name {
        "minecraft:suspicious_sand" => Some("minecraft:sand"),
        "minecraft:suspicious_gravel" => Some("minecraft:gravel"),
        _ => None,
    }
}

fn progress(count: i32) -> &'static str {
    match count {
        ..=0 => "0",
        1..=2 => "1",
        3..=5 => "2",
        _ => "3",
    }
}

/// A click with a brush held: true when it was one, whether or not it brushed. A press (the
/// client's first click) is left to the usual click, which shows the client the cell again.
pub(super) fn stroke(world: &mut World, player: Entity, pos: BlockPos, face: Face, click: &TransactionUseItem) -> bool {
    let held = world.get::<Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref()).is_some_and(|held| held.stack.name == BRUSH);
    if !held || click.trigger_type != TransactionUseItemTriggerType::SimulationTick {
        return false;
    }
    let Some(layer) = world.get::<InLayer>(player).map(|layer| layer.0).filter(|_| super::reaches(world, player, pos)) else { return true };
    let Some(level) = level(world, layer).filter(|level| level.in_height(pos[1])) else { return true };
    let id = level.block(pos[0], pos[1], pos[2]);
    if id == level.registry().air_id() {
        return true;
    }
    let name = level.registry().get(id).map(|state| state.name);
    // The sound names the block's first state, whatever its progress (bds-brush.txt).
    let first = name.and_then(|name| level.registry().states_of(name).next()).map_or(id, |(first, _)| first);
    let hash = level.registry().network_hash(first) as i32;
    if name.and_then(uncovered).is_some() && brush(world, layer, pos, face) {
        crate::inventory::wear_hand_by(world, player, 1, LASTS);
        return true;
    }
    let now = world.resource::<Tick>().0;
    let mut brushing = world.get_resource_or_insert_with(Brushing::default);
    if brushing.sounds.get(&player).is_some_and(|due| *due >= now) {
        return true;
    }
    brushing.sounds.insert(player, now + SOUND_EVERY);
    let [x, y, z] = pos.map(|c| c as f32 + 0.5);
    send::to_viewers(world, layer, pos, &[send::noise("brush", Vec3f { x, y, z }, hash)]);
    true
}

/// The block's entity as fields, made if it has none.
fn fields_at(world: &World, layer: Entity, pos: BlockPos) -> Vec<(acacia_proto::nbt::Str, Value)> {
    let entity = level(world, layer).and_then(|level| level.block_entity(pos)).map_or_else(|| new_block_entity(ENTITY, pos), |entity| entity.nbt.clone());
    match entity {
        Value::Compound(fields) => fields,
        _ => Vec::new(),
    }
}

fn int(fields: &[(acacia_proto::nbt::Str, Value)], key: &str) -> Option<i32> {
    fields.iter().find(|(name, _)| name == key).and_then(|(_, value)| match value {
        Value::Int(value) => Some(*value),
        Value::Byte(value) => Some((*value).into()),
        _ => None,
    })
}

fn put(fields: &mut Vec<(acacia_proto::nbt::Str, Value)>, key: &str, value: Value) {
    fields.retain(|(name, _)| name != key);
    fields.push((key.into(), value));
}

/// Keeps `fields` as the block's entity; when the count moved `brushed_progress` on, the block and
/// its entity are shown, as BDS shows them at no other count.
fn keep(world: &mut World, layer: Entity, pos: BlockPos, mut fields: Vec<(acacia_proto::nbt::Str, Value)>) {
    fields.sort_by(|a, b| a.0.cmp(&b.0));
    let count = int(&fields, COUNT).unwrap_or(0);
    let Some(level) = level(world, layer) else { return };
    let id = level.block(pos[0], pos[1], pos[2]);
    let moved = super::with_property(level.registry(), id, "brushed_progress", progress(count)).filter(|set| *set != id);
    match moved {
        Some(set) => {
            send::set(world, layer, pos, set);
            keep_entity(world, layer, pos, &Value::Compound(fields));
        }
        None => {
            if let Some(mut layer) = world.get_mut::<crate::components::Layer>(layer) {
                layer.world.set_block_entity(pos, &Value::Compound(fields));
            }
        }
    }
}

/// One stroke on the brushable block at `pos` from `face`; true when it was the last.
fn brush(world: &mut World, layer: Entity, pos: BlockPos, face: Face) -> bool {
    let now = world.resource::<Tick>().0;
    let mut brushing = world.get_resource_or_insert_with(Brushing::default);
    let strokes = brushing.blocks.entry((layer, pos)).or_default();
    if strokes.next_due.is_some_and(|due| now <= due) {
        return false;
    }
    // The next stroke is due STROKE after this one was: after a pause strokes come every tick until it catches up.
    strokes.next_due = Some(strokes.next_due.unwrap_or(now) + STROKE);
    (strokes.reset_at, strokes.resetting) = (now + RESET_AFTER, true);
    brushing.checks.push((now + RESET_AFTER, layer, pos));
    let mut fields = fields_at(world, layer, pos);
    if int(&fields, DIRECTION).is_none_or(|direction| direction == i32::from(NO_FACE)) {
        put(&mut fields, DIRECTION, Value::Byte(face as i8));
    }
    unpack(world, layer, &mut fields);
    let count = int(&fields, COUNT).unwrap_or(0) + 1;
    put(&mut fields, COUNT, Value::Int(count));
    if count >= STROKES {
        uncover(world, layer, pos, &fields);
        return true;
    }
    keep(world, layer, pos, fields);
    false
}

/// Rolls the block's `LootTable` into its `item` (`Count`, `Damage`, `Name`, `WasPickedUp`) once.
fn unpack(world: &mut World, layer: Entity, fields: &mut Vec<(acacia_proto::nbt::Str, Value)>) {
    let Some(table) = fields.iter().find_map(|(name, value)| match value {
        Value::String(table) if name == "LootTable" => Some(table.to_string()),
        _ => None,
    }) else { return };
    let seed = match int(fields, "LootTableSeed") {
        Some(seed) if seed != 0 => seed as u32,
        _ => world.get_resource_or_insert_with(super::Dice::default).roll(u32::MAX),
    };
    fields.retain(|(name, _)| name != "LootTable" && name != "LootTableSeed");
    let rolled = crate::containers::loot::rolled(&table, seed, vec![0]).into_iter().next();
    let Some(((name, count), level)) = rolled.map(|(_, item)| item).zip(level(world, layer)) else { return };
    let mut item = ItemStack { name, count: count.min(64) as u8, damage: 0, block: None, tag: None }.fields(level.registry());
    item.push(("WasPickedUp".into(), Value::Byte(0)));
    put(fields, ITEM, Value::Compound(item));
}

/// The last stroke: dust at the block's corner, the plain block, the item beside the face first
/// brushed, held still, and `brush_completed` naming the brushed block.
fn uncover(world: &mut World, layer: Entity, pos: BlockPos, fields: &[(acacia_proto::nbt::Str, Value)]) {
    world.get_resource_or_insert_with(Brushing::default).blocks.remove(&(layer, pos));
    let Some(level) = level(world, layer) else { return };
    let id = level.block(pos[0], pos[1], pos[2]);
    let hash = level.registry().network_hash(id) as i32;
    let plain = level.registry().get(id).and_then(|state| uncovered(state.name)).and_then(|name| level.registry().states_of(name).next()).map(|(id, _)| id);
    let Some(plain) = plain else { return };
    let [x, y, z] = pos.map(|c| c as f32);
    let dust = sapling_net::encode(&LevelEvent { event: LevelEventEvent::ParticleDestroyBlockNoSound, position: Vec3f { x, y, z }, data: hash });
    send::to_viewers(world, layer, pos, &[dust]);
    send::set(world, layer, pos, plain);
    let item = fields.iter().find(|(name, _)| name == ITEM).and_then(|(_, item)| ItemStack::parse(item));
    let face = int(fields, DIRECTION).and_then(|face| u32::try_from(face).ok()).and_then(Face::from_index).unwrap_or(Face::Up);
    if let Some(item) = item {
        let [nx, ny, nz] = face.neighbour(pos).map(|c| c as f32 + 0.5);
        // BDS spawns the item with its feet at the cell's middle; `eject` takes the box's middle.
        crate::drops::eject(world, layer, item, [nx, ny + 0.125, nz], [0.0; 3]);
    }
    send::to_viewers(world, layer, pos, &[send::noise("brush_completed", Vec3f { x: x + 0.5, y: y + 0.5, z: z + 0.5 }, hash)]);
}

/// Runs the checks of the count that are due: a block brushed within [`RESET_AFTER`] waits
/// [`RECHECK`] more; past it, the count falls by 2 every [`FALL_EVERY`] until it is 0.
pub(crate) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let Some(mut brushing) = world.get_resource_mut::<Brushing>() else { return };
    let (due, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut brushing.checks).into_iter().partition(|(at, _, _)| *at <= now);
    brushing.checks = waiting;
    for (_, layer, pos) in due {
        let is_brushable = level(world, layer).is_some_and(|level| level.registry().get(level.block(pos[0], pos[1], pos[2])).is_some_and(|state| uncovered(state.name).is_some()));
        let mut brushing = world.resource_mut::<Brushing>();
        let Some(strokes) = brushing.blocks.get_mut(&(layer, pos)).filter(|_| is_brushable) else { continue };
        if strokes.resetting {
            if now <= strokes.reset_at {
                brushing.checks.push((now + RECHECK, layer, pos));
                continue;
            }
            strokes.resetting = false;
        }
        let mut fields = fields_at(world, layer, pos);
        let count = int(&fields, COUNT).unwrap_or(0);
        if count == 0 {
            put(&mut fields, DIRECTION, Value::Byte(NO_FACE));
            keep(world, layer, pos, fields);
            continue;
        }
        put(&mut fields, COUNT, Value::Int((count - 2).max(0)));
        keep(world, layer, pos, fields);
        world.resource_mut::<Brushing>().checks.push((now + FALL_EVERY, layer, pos));
    }
}
