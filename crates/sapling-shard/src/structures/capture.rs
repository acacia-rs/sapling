//! A box of a world taken as a structure template: its blocks and liquids by palette, what they
//! hold, and the kept entities whose feet are inside.
// TODO: an entity the shard moves but keeps no compound of (a plugin's statue) is left out.

use std::collections::HashMap;

use acacia_proto::nbt::{self, LittleEndian, Value};
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::{BlockRegistry, ChunkPos, StructureTemplate, block_of_hash};

use crate::actors::compound_now;
use crate::body::Body;
use crate::components::{InLayer, Layer};
use crate::creatures::Health;
use crate::entities::Stored;

/// The box from `low`, `size` cells along each axis, as a template; `None` where a column of it
/// is not loaded.
pub(crate) fn capture(world: &mut World, layer: Entity, low: BlockPos, size: [i32; 3], include_entities: bool, include_blocks: bool) -> Option<StructureTemplate> {
    let level = &world.get::<Layer>(layer)?.world;
    let columns = (low[0] >> 4..=(low[0] + size[0] - 1) >> 4).flat_map(|x| (low[2] >> 4..=(low[2] + size[2] - 1) >> 4).map(move |z| ChunkPos::new(x, z)));
    if !columns.into_iter().all(|column| level.is_loaded(column)) {
        return None;
    }
    let mut template = StructureTemplate { size, origin: low, palette: Vec::new(), layers: [Vec::new(), Vec::new()], block_entities: Vec::new(), entities: Vec::new() };
    let volume = template.volume();
    template.layers = [vec![-1; volume], vec![-1; volume]];
    if include_blocks {
        let mut palette = Palette::new(level.registry());
        for index in 0..volume {
            let offset = template.offset(index);
            let [x, y, z] = std::array::from_fn(|axis| low[axis] + offset[axis]);
            if !level.in_height(y) {
                continue;
            }
            template.layers[0][index] = palette.index(level.block(x, y, z));
            let liquid = level.liquid(x, y, z);
            if liquid != level.registry().air_id() {
                template.layers[1][index] = palette.index(liquid);
            }
            if let Some(entity) = level.block_entity([x, y, z]) {
                template.block_entities.push((index, entity.nbt.clone()));
            }
        }
        template.palette = palette.entries;
    }
    if include_entities {
        template.entities = entities_in(world, layer, low, size);
    }
    Some(template)
}

/// The kept entities of `layer` whose feet are in the box, as their stored compounds now.
fn entities_in(world: &mut World, layer: Entity, low: BlockPos, size: [i32; 3]) -> Vec<Value> {
    let inside = |at: [f32; 3]| (0..3).all(|axis| at[axis] >= low[axis] as f32 && at[axis] < (low[axis] + size[axis]) as f32);
    world
        .query::<(&InLayer, &Body, &Stored, Option<&Health>)>()
        .iter(world)
        .filter(|(in_layer, body, ..)| in_layer.0 == layer && inside(body.at))
        .filter_map(|(_, body, stored, health)| compound_now(body, stored, health))
        .filter_map(|compound| nbt::read::<LittleEndian>(&mut &compound[..]).ok().map(|read| read.value))
        .collect()
}

/// The palette a template is built with, each state once.
struct Palette<'a> {
    registry: &'a BlockRegistry,
    entries: Vec<Value>,
    by_id: HashMap<u32, i32>,
}

impl<'a> Palette<'a> {
    fn new(registry: &'a BlockRegistry) -> Self {
        Self { registry, entries: Vec::new(), by_id: HashMap::new() }
    }

    /// The palette index of the state `id`; -1 for a state no entry can name.
    fn index(&mut self, id: u32) -> i32 {
        if let Some(&index) = self.by_id.get(&id) {
            return index;
        }
        let index = match block_of_hash(self.registry, self.registry.network_hash(id)) {
            Some(entry) => {
                self.entries.push(entry);
                self.entries.len() as i32 - 1
            }
            None => -1,
        };
        self.by_id.insert(id, index);
        index
    }
}
