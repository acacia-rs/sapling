//! Maps kept with the world as BDS keeps them (`map_<id>`, sapling_leveldb::MapRecord): each realm's
//! maps are read from its level the first time they are needed, and handed back when it saves.

use std::sync::Arc;

use bevy_ecs::prelude::*;
use sapling_world::MapRecord;

use super::{ID_BASE, IDS_PER_MAP, MapData, Maps};
use crate::components::Layer;
use crate::dimensions::{home, members};

/// The highest scale BDS makes a map's run up to: its record names no parent.
const TOP_SCALE: u8 = 4;

/// The maps of `layer`'s realm, read from its level the first time.
pub(super) fn maps_mut(world: &mut World, layer: Entity) -> Mut<'_, Maps> {
    let home = home(world, layer);
    if world.get::<Maps>(home).is_none() {
        let maps = restored(world, home);
        world.entity_mut(home).insert(maps);
    }
    world.get_mut::<Maps>(home).expect("inserted above")
}

/// Reads the maps of every realm that has not yet, so that a table can be shown them.
pub(super) fn restore_all(world: &mut World) {
    let homes: Vec<Entity> = world.query_filtered::<Entity, (With<Layer>, Without<Maps>)>().iter(world).filter(|layer| home(world, *layer) == *layer).collect();
    for layer in homes {
        let maps = restored(world, layer);
        world.entity_mut(layer).insert(maps);
    }
}

fn restored(world: &mut World, home: Entity) -> Maps {
    let layers: Vec<(i32, Entity)> = members(world, home).into_iter().filter_map(|layer| Some((world.get::<Layer>(layer)?.world.dimension(), layer))).collect();
    let records = world.get_mut::<Layer>(home).map(|mut level| std::mem::take(&mut level.world.level_mut().maps)).unwrap_or_default();
    let mut maps = Maps::default();
    for record in records.iter() {
        let dimension = i32::from(record.dimension);
        let layer = layers.iter().find(|(kept, _)| *kept == dimension).map_or(home, |(_, layer)| *layer);
        let pixels = record.colours.chunks_exact(4).map(|rgba| u32::from_le_bytes([rgba[0], rgba[1], rgba[2], rgba[3]])).collect();
        let map = MapData::kept(record.id, record.scale as u8, record.centre, dimension, layer, record.locked, pixels);
        maps.maps.insert(record.id, map);
    }
    let runs = records.iter().filter(|record| record.id > ID_BASE && record.id < 0).map(|record| (record.id - ID_BASE - 1) / IDS_PER_MAP + 1).max();
    // TODO: BDS's own count of `map_name_index` was not found; a restarted world counts on from its runs.
    (maps.made, maps.names) = (runs.unwrap_or(0), runs.unwrap_or(0) as i32);
    maps
}

/// Hands each realm's maps to its level to be saved with it.
pub(crate) fn keep_all(world: &mut World) {
    let kept: Vec<(Entity, Vec<MapRecord>)> = world.query::<(Entity, &Maps)>().iter(world).map(|(home, maps)| (home, maps.maps.values().map(record).collect())).collect();
    for (home, records) in kept {
        if let Some(mut layer) = world.get_mut::<Layer>(home) {
            layer.world.level_mut().maps = Arc::new(records);
        }
    }
}

fn record(map: &MapData) -> MapRecord {
    let parent = if map.scale < TOP_SCALE && !map.locked { map.id + 1 } else { -1 };
    let colours = map.pixels.iter().flat_map(|pixel| pixel.to_le_bytes()).collect();
    MapRecord { id: map.id, parent, dimension: map.dimension as i8, scale: map.scale as i8, locked: map.locked, centre: map.centre, colours }
}
