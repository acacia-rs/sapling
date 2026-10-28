//! Nether portals as blocks, as BDS's `PortalBlock` and `PortalShape` keep them
//! (docs/research/nether-dimension.md): fire in a whole obsidian frame of the overworld or the
//! Nether fills it with portal blocks, and a change beside one that leaves its frame broken or
//! its inside part filled empties the whole sheet. Both go out as BDS sends them, one
//! UpdateSubchunkBlocks per section. Each lit portal is kept as a record its world's portals
//! find it by (portals.rs).

pub(crate) mod build;
pub(crate) mod shape;

use acacia_proto::packets::UpdateSubchunkBlocks;
use acacia_proto::types::{BlockCoordinates, BlockUpdate, BlockUpdateTransitionType};
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::{NETHER, OVERWORLD, PortalRecord};
use shape::{Axis, Cell, Shape};

use super::{level, send};
use crate::components::Layer;

pub(crate) const PORTAL: &str = "minecraft:portal";
const OBSIDIAN: &str = "minecraft:obsidian";
const FIRE: &str = "minecraft:fire";
/// `UpdateBlock` flags BDS gave a portal's cells: the network bit alone.
const NETWORK: u32 = 2;
const SIDES: [[i32; 3]; 6] = [[0, -1, 0], [0, 1, 0], [-1, 0, 0], [1, 0, 0], [0, 0, -1], [0, 0, 1]];

/// Cells written this tick that may light or break a portal, checked by [`settle`].
#[derive(Resource, Default)]
pub(crate) struct Touched(Vec<(Entity, BlockPos)>);

/// What a cell is to a frame in a world.
pub(crate) fn cell(level: &sapling_world::World, [x, y, z]: BlockPos) -> Cell {
    let loaded = level.in_height(y) && level.is_loaded(sapling_world::ChunkPos::of_block(x, z));
    match loaded.then(|| level.registry().get(level.block(x, y, z))).flatten().map(|state| state.name) {
        Some(OBSIDIAN) => Cell::Frame,
        Some("minecraft:air" | FIRE | PORTAL) => Cell::Open,
        _ => Cell::Other,
    }
}

fn is_portal(level: &sapling_world::World, [x, y, z]: BlockPos) -> bool {
    level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| state.name == PORTAL)
}

/// The portal state of an axis.
pub(crate) fn portal_id(level: &sapling_world::World, axis: Axis) -> Option<u32> {
    level.registry().find(PORTAL, &format!("portal_axis={}", axis.name()))
}

/// Lights the frame round `at`, a cell fire is to go in: its inside becomes portal blocks, no
/// fire is placed and its record is kept. False, with nothing done, outside a whole frame, in a
/// frame that holds portal blocks already and in the End.
pub(crate) fn kindle(world: &mut World, layer: Entity, at: BlockPos) -> bool {
    let Some(level) = level(world, layer).filter(|level| [OVERWORLD, NETHER].contains(&level.dimension())) else { return false };
    let Some(shape) = shape::find(at, &|pos| cell(level, pos)).filter(|shape| !shape.cells().iter().any(|pos| is_portal(level, *pos))) else { return false };
    let Some(id) = portal_id(level, shape.axis) else { return false };
    let cells: Vec<(BlockPos, u32)> = shape.cells().into_iter().map(|pos| (pos, id)).collect();
    if !crate::world_events::may_create_portal(world, layer, &cells) {
        return false;
    }
    write_shown(world, layer, &cells);
    keep(world, layer, &shape);
    true
}

/// Writes `cells` without telling anyone, then shows them to those who see them, in their order,
/// one UpdateSubchunkBlocks per section.
pub(crate) fn write_shown(world: &mut World, layer: Entity, cells: &[(BlockPos, u32)]) {
    write_shown_flagged(world, layer, cells, NETWORK);
}

/// [`write_shown`] with the updates' flags.
pub(crate) fn write_shown_flagged(world: &mut World, layer: Entity, cells: &[(BlockPos, u32)], flags: u32) {
    let written: Vec<(BlockPos, u32)> = cells.iter().copied().filter(|(pos, id)| send::set_quietly(world, layer, *pos, *id)).collect();
    show_in_sections(world, layer, &written, flags);
}

/// Shows cells already written as [`write_shown`] does.
pub(crate) fn show_in_sections(world: &mut World, layer: Entity, written: &[(BlockPos, u32)], flags: u32) {
    let Some(registry) = level(world, layer).map(|level| level.registry()) else { return };
    let mut sections: Vec<([i32; 3], Vec<BlockUpdate>)> = Vec::new();
    for &([x, y, z], id) in written {
        let section = [x >> 4, y >> 4, z >> 4];
        let update = BlockUpdate { position: BlockCoordinates { x, y, z }, runtime_id: registry.network_hash(id), flags, entity_unique_id: 0, transition_type: BlockUpdateTransitionType::Entity };
        match sections.iter_mut().find(|(at, _)| *at == section) {
            Some((_, updates)) => updates.push(update),
            None => sections.push((section, vec![update])),
        }
    }
    for ([sx, sy, sz], blocks) in sections {
        let corner = [sx * 16, sy * 16, sz * 16];
        let packet = sapling_net::encode(&UpdateSubchunkBlocks { x: corner[0], y: corner[1], z: corner[2], blocks, extra: Vec::new() });
        send::to_viewers(world, layer, corner, &[packet]);
    }
}

/// Keeps a lit portal's record with its world's level.
pub(crate) fn keep(world: &mut World, layer: Entity, shape: &Shape) {
    let home = crate::dimensions::home(world, layer);
    let Some(dimension) = level(world, layer).map(|level| level.dimension()) else { return };
    let record = PortalRecord { dimension, corner: shape.corner, span: shape.width as u8, along_x: shape.axis == Axis::X };
    if let Some(mut level) = world.get_mut::<Layer>(home) {
        let portals = &mut level.world.level_mut().portals;
        if !portals.contains(&record) {
            portals.push(record);
        }
    }
    crate::dimensions::share_levels(world);
}

/// Notes a written cell: fire there may light a frame, and a portal beside it may be broken.
pub(super) fn written(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(level) = level(world, layer) else { return };
    let [x, y, z] = pos;
    let is_fire = level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| state.name == FIRE);
    if is_fire || SIDES.iter().any(|[dx, dy, dz]| is_portal(level, [x + dx, y + dy, z + dz])) {
        world.get_resource_or_insert_with(Touched::default).0.push((layer, pos));
    }
}

/// Lights the frames fire was put in this tick and empties the portals whose frame a change broke.
pub(crate) fn settle(world: &mut World) {
    let Some(touched) = world.get_resource_mut::<Touched>().map(|mut touched| std::mem::take(&mut touched.0)) else { return };
    for (layer, [x, y, z]) in touched {
        let is_fire = level(world, layer).is_some_and(|level| level.registry().get(level.block(x, y, z)).is_some_and(|state| state.name == FIRE));
        if is_fire && kindle(world, layer, [x, y, z]) {
            continue;
        }
        for [dx, dy, dz] in SIDES {
            break_unframed(world, layer, [x + dx, y + dy, z + dz]);
        }
    }
}

/// Empties the portal sheet at `pos` if its frame is no longer whole or its inside is not all
/// portal (BDS `PortalBlock::neighborChanged`): the whole sheet at once, top row first, and its
/// record goes.
fn break_unframed(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(level) = level(world, layer) else { return };
    let Some(axis) = portal_axis(level, pos) else { return };
    let cell_at = |at: BlockPos| cell(level, at);
    let whole = shape::find_along(pos, axis, &cell_at).filter(|shape| shape.contains(pos) && shape.cells().iter().all(|at| is_portal(level, *at)));
    if whole.is_some() {
        return;
    }
    let sheet = sheet(level, pos, axis);
    let air = level.registry().air_id();
    let corner = |sheet: &[BlockPos]| sheet.iter().copied().min_by_key(|[x, y, z]| (*y, *x, *z));
    let lowest = corner(&sheet);
    let mut cells: Vec<(BlockPos, u32)> = sheet.into_iter().map(|at| (at, air)).collect();
    cells.sort_by_key(|([x, y, z], _)| (-*y, *x, *z));
    write_shown(world, layer, &cells);
    let (Some(dimension), Some(home)) = (level_dimension(world, layer), Some(crate::dimensions::home(world, layer))) else { return };
    if let (Some(lowest), Some(mut level)) = (lowest, world.get_mut::<Layer>(home)) {
        level.world.level_mut().portals.retain(|record| !(record.dimension == dimension && record.corner == lowest));
    }
    crate::dimensions::share_levels(world);
}

fn level_dimension(world: &World, layer: Entity) -> Option<i32> {
    level(world, layer).map(|level| level.dimension())
}

/// What a cell is to a portal site (build.rs): air, or a plant or the like a portal may replace
/// over what is no liquid, is empty (BDS's site check since its replace fix).
// TODO: BDS asks the block whether it can be replaced; this list is the plants and fire it knows.
pub(crate) fn ground(level: &sapling_world::World, [x, y, z]: BlockPos) -> build::Ground {
    let state = |y: i32| (level.in_height(y) && level.is_loaded(sapling_world::ChunkPos::of_block(x, z))).then(|| level.registry().get(level.block(x, y, z))).flatten();
    let Some(here) = state(y) else { return build::Ground::Other };
    let replaceable = REPLACEABLE.iter().any(|name| here.name.strip_prefix("minecraft:") == Some(*name));
    match () {
        _ if here.is_air() => build::Ground::Empty,
        _ if replaceable && !here.is_liquid() && state(y - 1).is_some_and(|below| !below.is_liquid()) => build::Ground::Empty,
        _ if here.is_solid() => build::Ground::Solid,
        _ => build::Ground::Other,
    }
}

const REPLACEABLE: [&str; 13] = ["short_grass", "tall_grass", "fern", "large_fern", "deadbush", "vine", "crimson_roots", "warped_roots", "nether_sprouts", "glow_lichen", "fire", "soul_fire", "snow_layer"];

pub(crate) fn portal_axis(level: &sapling_world::World, [x, y, z]: BlockPos) -> Option<Axis> {
    let state = level.in_height(y).then(|| level.registry().get(level.block(x, y, z))).flatten().filter(|state| state.name == PORTAL)?;
    Some(if state.properties.contains("portal_axis=z") { Axis::Z } else { Axis::X })
}

/// The portal blocks of one axis joined to `pos` in its plane, at most a frame's worth.
fn sheet(level: &sapling_world::World, pos: BlockPos, axis: Axis) -> Vec<BlockPos> {
    let steps: [[i32; 3]; 4] = match axis {
        Axis::X => [[1, 0, 0], [-1, 0, 0], [0, 1, 0], [0, -1, 0]],
        Axis::Z => [[0, 0, 1], [0, 0, -1], [0, 1, 0], [0, -1, 0]],
    };
    let (mut found, mut next) = (vec![pos], vec![pos]);
    while let Some([x, y, z]) = next.pop() {
        for [dx, dy, dz] in steps {
            let at = [x + dx, y + dy, z + dz];
            if found.len() < (shape::MAX * shape::MAX) as usize && !found.contains(&at) && portal_axis(level, at) == Some(axis) {
                found.push(at);
                next.push(at);
            }
        }
    }
    found
}
