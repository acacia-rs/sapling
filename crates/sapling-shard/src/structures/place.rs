//! A structure template put into a world as `/structure load` puts it, all at once: its blocks
//! in one batch (BDS sent one UpdateSubchunkBlocks), what they hold marked `forceunpair` as BDS
//! marked a chest, its liquids left to flow, and its entities as new ones turned with the box
//! (research/captures/bds-structures.txt).
// TODO: BDS's `layer_by_layer` and `block_by_block` animations place over time; here they are instant.

use acacia_proto::nbt::{self, LittleEndian, Nbt, Value};
use bevy_ecs::prelude::*;
use bytes::BytesMut;
use sapling_api::{Block, BlockPos, StructureLoad};
use sapling_world::{ChunkPos, StructureTemplate, palette_block, placed_actor, stored_actors};

use super::turn::Turn;
use crate::components::{Layer, Players, Tick};
use crate::{actors, blocks};

/// Places `template` with the lowest corner of its turned box at `at`.
pub(crate) fn place(world: &mut World, layer: Entity, template: &StructureTemplate, at: BlockPos, how: &StructureLoad) {
    let turn = Turn::of(how.rotation, how.mirror);
    if how.include_blocks {
        place_blocks(world, layer, template, at, turn, how);
    }
    if how.include_entities {
        place_entities(world, layer, template, at, turn);
    }
}

fn place_blocks(world: &mut World, layer: Entity, template: &StructureTemplate, at: BlockPos, turn: Turn, how: &StructureLoad) {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let ids: Vec<Option<u32>> = template.palette.iter().map(|entry| {
        let (name, properties) = palette_block(entry)?;
        let properties = turn.states(&name, &properties);
        blocks::resolve(level.registry(), &Block { name, properties })
    }).collect();
    let mut dice = Integrity::new(&how.seed, world.resource::<Tick>().0);
    let kept: Vec<usize> = (0..template.volume()).filter(|_| how.integrity >= 1.0 || dice.fraction() < how.integrity).collect();
    let cell = |index: usize| -> BlockPos {
        let offset = turn.cell(template.offset(index), template.size);
        std::array::from_fn(|axis| at[axis] + offset[axis])
    };
    let id_of = |layer: usize, index: usize| usize::try_from(template.layers[layer][index]).ok().and_then(|palette| ids.get(palette).copied().flatten());
    blocks::batched(world, |world| {
        for &index in &kept {
            if let Some(id) = id_of(0, index) {
                blocks::set(world, layer, cell(index), id);
            }
        }
    });
    let air = world.get::<Layer>(layer).map(|layer| layer.world.registry().air_id());
    for &index in &kept {
        let pos = cell(index);
        match (id_of(1, index), air) {
            (Some(liquid), _) => drop(blocks::set_liquid(world, layer, pos, liquid)),
            (None, Some(air)) if !how.waterlogged && id_of(0, index).is_some() => drop(blocks::set_liquid(world, layer, pos, air)),
            _ => {}
        }
    }
    for (index, entity) in &template.block_entities {
        if kept.binary_search(index).is_ok() {
            let pos = cell(*index);
            blocks::keep_entity(world, layer, pos, &unpaired(blocks::block_entity_at(entity.clone(), pos)));
        }
    }
    for &index in &kept {
        blocks::neighbours_changed(world, layer, cell(index));
    }
}

/// A chest's block entity no longer pairs with the one it was beside when it was saved.
fn unpaired(entity: Value) -> Value {
    let Value::Compound(mut fields) = entity else { return entity };
    if fields.iter().any(|(key, value)| key == "id" && matches!(value, Value::String(id) if id.as_str() == "Chest")) {
        fields.retain(|(key, _)| key != "forceunpair");
        fields.push(("forceunpair".into(), Value::Byte(1)));
    }
    Value::Compound(fields)
}

fn place_entities(world: &mut World, layer: Entity, template: &StructureTemplate, at: BlockPos, turn: Turn) {
    let floats = |value: Option<&Value>| -> Option<Vec<f32>> {
        let Some(Value::List(list)) = value else { return None };
        list.items.iter().map(|item| if let Value::Float(f) = item { Some(*f) } else { None }).collect()
    };
    let high = (world.resource::<Tick>().0 as i64) << 32;
    for (n, entity) in template.entities.iter().enumerate() {
        let Some(pos) = floats(entity.get("Pos")).filter(|pos| pos.len() == 3) else { continue };
        let [yaw, pitch] = floats(entity.get("Rotation")).and_then(|rotation| rotation.try_into().ok()).unwrap_or([0.0, 0.0]);
        let local: [f32; 3] = std::array::from_fn(|axis| pos[axis] - template.origin[axis] as f32);
        let turned = turn.point(local, template.size);
        let feet: [f32; 3] = std::array::from_fn(|axis| at[axis] as f32 + turned[axis]);
        let unique_id = high | ((world.resource::<Players>().next_net_id as i64 + n as i64) & 0xffff_ffff);
        let mut bytes = BytesMut::new();
        nbt::write::<LittleEndian>(&mut bytes, &Nbt { name: String::new(), value: entity.clone() });
        let Ok(compound) = placed_actor(&bytes, unique_id, feet, [turn.yaw(yaw), pitch]) else { continue };
        let Ok(mut read) = stored_actors(&compound) else { continue };
        let Some((actor, unique_id, compound)) = read.pop() else { continue };
        let home = ChunkPos::of_block(feet[0].floor() as i32, feet[2].floor() as i32);
        actors::bring_in(world, layer, home, actor, unique_id, compound);
    }
}

/// Which cells a load of integrity below 1 places: the same ones for the same seed, as BDS
/// placed the same half of a box twice under one seed.
// TODO: BDS's own draw from a seed is not known; these are sapling's.
struct Integrity(u64);

impl Integrity {
    fn new(seed: &str, tick: u64) -> Self {
        let hashed = if seed.is_empty() { tick } else { seed.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)) };
        Self(hashed | 1)
    }

    fn fraction(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
}
