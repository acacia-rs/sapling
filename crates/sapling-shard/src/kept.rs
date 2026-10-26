//! Columns kept loaded, and so ticking, with no player near: those of a world's ticking areas,
//! which its level keeps on disk, and those a plugin forces. Bedrock keeps no spawn chunks; only
//! these stay (docs/research/structures-and-ticking.md).

use std::collections::{BTreeSet, HashSet};

use bevy_ecs::prelude::*;
use sapling_api::{MOST_TICKING_AREAS, TickingArea, TickingBackend};
use sapling_world::{ChunkPos, TickingAreaRecord};

use crate::components::{Layer, Tick};
use crate::handlers::Ctx;
use crate::{dimensions, worlds};

/// How often the kept columns are counted again and asked for.
const KEEP_INTERVAL: u64 = 10;

/// What a world keeps loaded: the chunks plugins force, and with its ticking areas every chunk.
#[derive(Component, Default)]
pub(crate) struct Kept {
    forced: BTreeSet<ChunkPos>,
    all: HashSet<ChunkPos>,
}

impl Kept {
    pub(crate) fn holds(&self, pos: ChunkPos) -> bool {
        self.all.contains(&pos)
    }
}

fn area(record: &TickingAreaRecord) -> TickingArea {
    TickingArea { name: record.name.clone(), min: record.min.map(|block| block >> 4), max: record.max.map(|block| block >> 4), circle: record.circle, preload: record.preload }
}

/// The record of a new area; its key is made from the area, its world and when it came.
fn record(area: &TickingArea, dimension: i32, tick: u64) -> TickingAreaRecord {
    let hash = format!("{}{dimension}{tick}{:?}{:?}", area.name, area.min, area.max).bytes().fold(0xcbf2_9ce4_8422_2325u64, |hash, byte| (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3));
    let id = format!("{:08x}-{:04x}-4{:03x}-8{:03x}-{:012x}", hash >> 32, (hash >> 16) & 0xffff, hash & 0xfff, (hash >> 20) & 0xfff, hash.rotate_left(17) & 0xffff_ffff_ffff);
    TickingAreaRecord { id, dimension, name: area.name.clone(), circle: area.circle, min: area.min.map(|chunk| chunk * 16), max: area.max.map(|chunk| chunk * 16 + 15), preload: area.preload }
}

/// The ticking area records of `layer`'s realm, and the dimension `layer` is.
fn records(world: &mut World, layer: Entity) -> Option<(Mut<'_, Layer>, i32)> {
    let dimension = world.get::<Layer>(layer)?.world.dimension();
    let home = dimensions::home(world, layer);
    Some((world.get_mut::<Layer>(home)?, dimension))
}

fn areas_of(world: &World, layer: Entity) -> Vec<TickingArea> {
    let Some(dimension) = world.get::<Layer>(layer).map(|level| level.world.dimension()) else { return Vec::new() };
    let Some(home) = world.get::<Layer>(dimensions::home(world, layer)) else { return Vec::new() };
    home.world.level().ticking_areas.iter().filter(|record| record.dimension == dimension).map(area).collect()
}

/// Counts every world's kept columns anew and asks for those not loaded.
pub(crate) fn keep_columns(world: &mut World) {
    if world.resource::<Tick>().0 % KEEP_INTERVAL != 0 {
        return;
    }
    let layers: Vec<Entity> = world.query_filtered::<Entity, With<Layer>>().iter(world).collect();
    for layer in layers {
        let areas = areas_of(world, layer);
        let forced = world.get::<Kept>(layer).map(|kept| kept.forced.clone()).unwrap_or_default();
        if areas.is_empty() && forced.is_empty() && world.get::<Kept>(layer).is_none() {
            continue;
        }
        let mut all: HashSet<ChunkPos> = forced.iter().copied().collect();
        for area in &areas {
            let chunks = (area.min[0]..=area.max[0]).flat_map(|x| (area.min[1]..=area.max[1]).map(move |z| [x, z]));
            all.extend(chunks.filter(|chunk| area.holds(*chunk)).map(|[x, z]| ChunkPos::new(x, z)));
        }
        if let Some(mut level) = world.get_mut::<Layer>(layer) {
            all.iter().for_each(|pos| _ = level.world.request(*pos));
        }
        world.entity_mut(layer).insert(Kept { forced, all });
    }
}

impl TickingBackend for Ctx<'_> {
    fn ticking_areas(&self) -> Vec<TickingArea> {
        areas_of(self.world, worlds::current(self.world))
    }

    fn add_ticking_area(&mut self, area: TickingArea) -> bool {
        let (layer, tick) = (worlds::current(self.world), self.world.resource::<Tick>().0);
        let Some((mut home, dimension)) = records(self.world, layer) else { return false };
        let records = &mut home.world.level_mut().ticking_areas;
        if records.iter().filter(|record| record.dimension == dimension).count() >= MOST_TICKING_AREAS {
            return false;
        }
        records.push(record(&area, dimension, tick));
        true
    }

    fn remove_ticking_areas(&mut self, pick: &dyn Fn(&TickingArea) -> bool) -> Vec<TickingArea> {
        let layer = worlds::current(self.world);
        let Some((mut home, dimension)) = records(self.world, layer) else { return Vec::new() };
        let records = &mut home.world.level_mut().ticking_areas;
        let mut removed = Vec::new();
        records.retain(|record| {
            let picked = record.dimension == dimension && pick(&area(record));
            if picked {
                removed.push(area(record));
            }
            !picked
        });
        removed
    }

    fn set_ticking_preload(&mut self, pick: &dyn Fn(&TickingArea) -> bool, preload: bool) -> usize {
        let layer = worlds::current(self.world);
        let Some((mut home, dimension)) = records(self.world, layer) else { return 0 };
        let picked = home.world.level_mut().ticking_areas.iter_mut().filter(|record| record.dimension == dimension && pick(&area(record)));
        picked.map(|record| record.preload = preload).count()
    }

    fn force_chunk(&mut self, [x, z]: [i32; 2], forced: bool) {
        let layer = worlds::current(self.world);
        if self.world.get::<Kept>(layer).is_none() {
            self.world.entity_mut(layer).insert(Kept::default());
        }
        let Some(mut kept) = self.world.get_mut::<Kept>(layer) else { return };
        let pos = ChunkPos::new(x, z);
        if forced {
            kept.forced.insert(pos);
            kept.all.insert(pos);
        } else {
            kept.forced.remove(&pos);
        }
    }

    fn forced_chunks(&self) -> Vec<[i32; 2]> {
        let layer = worlds::current(self.world);
        self.world.get::<Kept>(layer).map(|kept| kept.forced.iter().map(|pos| [pos.x, pos.z]).collect()).unwrap_or_default()
    }
}
