//! A structure template put into a world as `/structure load` puts it: its blocks in batches
//! (BDS sent one UpdateSubchunkBlocks for a load at once), what they hold marked `forceunpair` as
//! BDS marked a chest, its liquids left to flow, and its entities as new ones turned with the box
//! (research/captures/bds-structures.txt). An animated load puts its cells over time (animate.rs).

use std::collections::HashSet;

use acacia_proto::nbt::{self, LittleEndian, Nbt, Value};
use bevy_ecs::prelude::*;
use bytes::BytesMut;
use sapling_api::{Block, BlockPos, StructureLoad};
use sapling_world::{ChunkPos, StructureTemplate, palette_block, placed_actor, stored_actors};

use super::turn::Turn;
use crate::components::{Layer, Players, Tick};
use crate::{actors, blocks};

/// A template about to be put with the lowest corner of its turned box at `at`: its palette as
/// the world's states, and the cells its integrity keeps, in order.
pub(crate) struct Placing {
    template: StructureTemplate,
    at: BlockPos,
    turn: Turn,
    ids: Vec<Option<u32>>,
    pub kept: Vec<usize>,
    how: StructureLoad,
}

impl Placing {
    pub(crate) fn new(world: &World, layer: Entity, template: StructureTemplate, at: BlockPos, how: &StructureLoad) -> Option<Placing> {
        let level = &world.get::<Layer>(layer)?.world;
        let turn = Turn::of(how.rotation, how.mirror);
        let ids = template.palette.iter().map(|entry| {
            let (name, properties) = palette_block(entry)?;
            let properties = turn.states(&name, &properties);
            blocks::resolve(level.registry(), &Block { name, properties })
        }).collect();
        let mut dice = Integrity::new(&how.seed, world.resource::<Tick>().0);
        let kept = if how.include_blocks { (0..template.volume()).filter(|_| how.integrity >= 1.0 || dice.fraction() < how.integrity).collect() } else { Vec::new() };
        Some(Placing { template, at, turn, ids, kept, how: how.clone() })
    }

    pub(crate) fn template(&self) -> &StructureTemplate {
        &self.template
    }

    fn cell(&self, index: usize) -> BlockPos {
        let offset = self.turn.cell(self.template.offset(index), self.template.size);
        std::array::from_fn(|axis| self.at[axis] + offset[axis])
    }

    fn id_of(&self, layer: usize, index: usize) -> Option<u32> {
        usize::try_from(self.template.layers[layer][index]).ok().and_then(|palette| self.ids.get(palette).copied().flatten())
    }

    /// Puts the template's cells `cells` (of those kept) with their liquids and what they hold.
    /// BDS showed them in one list per sub-chunk: the cells that are not air, by y, x and z, then
    /// every cell in the template's order, changed or not; and each block entity twice
    /// (research/captures/bds-structblocks.txt).
    pub(crate) fn place_cells(&self, world: &mut World, layer: Entity, cells: &[usize]) {
        let air = world.get::<Layer>(layer).map(|layer| layer.world.registry().air_id());
        let written: Vec<(usize, u32)> = cells.iter().filter_map(|&index| Some((index, self.id_of(0, index)?))).collect();
        for &(index, id) in &written {
            blocks::set_quietly(world, layer, self.cell(index), id);
        }
        let mut solid: Vec<(usize, u32)> = written.iter().copied().filter(|&(_, id)| Some(id) != air).collect();
        solid.sort_by_key(|&(index, _)| {
            let [x, y, z] = self.template.offset(index);
            (y, x, z)
        });
        let mut in_order = written.clone();
        in_order.sort_unstable();
        let shown: Vec<(BlockPos, u32)> = solid.iter().chain(&in_order).map(|&(index, id)| (self.cell(index), id)).collect();
        blocks::show_states(world, layer, &shown);
        for &index in cells {
            let pos = self.cell(index);
            match (self.id_of(1, index), air) {
                (Some(liquid), _) => drop(blocks::set_liquid(world, layer, pos, liquid)),
                (None, Some(air)) if !self.how.waterlogged && self.id_of(0, index).is_some() => drop(blocks::set_liquid(world, layer, pos, air)),
                _ => {}
            }
        }
        let placed: HashSet<usize> = cells.iter().copied().collect();
        for (index, entity) in self.template.block_entities.iter().filter(|(index, _)| placed.contains(index)) {
            let pos = self.cell(*index);
            let entity = unpaired(blocks::block_entity_at(entity.clone(), pos));
            blocks::keep_entity(world, layer, pos, &entity);
            blocks::keep_entity(world, layer, pos, &entity);
        }
        for &index in cells {
            blocks::neighbours_changed(world, layer, self.cell(index));
        }
    }

    /// Brings in the template's entities, where the load includes them.
    pub(crate) fn place_entities(&self, world: &mut World, layer: Entity) {
        if !self.how.include_entities {
            return;
        }
        let (template, at, turn) = (&self.template, self.at, self.turn);
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
