//! Putting a portal's traveller in place, as BDS's `PortalForcer::force` (144783640) does once
//! the columns round where it arrives are loaded: in the portal whose record is nearest, at the
//! same fraction across it as it stood in the one it left, else in a portal made for it.
//! Then MovePlayer mode Reset and the word to leave the loading screen, then the new portal.

use bevy_ecs::prelude::*;
use sapling_data::EYE_HEIGHT;
use sapling_world::{ChunkPos, NETHER, PortalRecord};

use super::draws::Directions;
use super::records;
use crate::dimensions::Trip;
use crate::blocks::portals::{self as blocks, build, shape};
use crate::components::{InLayer, Layer, NetId, Outgoing, Position};
use crate::dimensions::{self, Arriving};
use crate::movement;
use crate::chunks;
use crate::view::View;

/// Columns this far round the place must be loaded before it is settled (BDS waits on 32 blocks).
const LOADED_REACH: i32 = 2;

/// The record of the portal a traveller left, and how far across it it stood, 0 to 1.
fn entrance(portals: &[PortalRecord], trip: &Trip) -> Option<(PortalRecord, f32)> {
    let (record, _) = records::nearest(portals, trip.from, trip.entrance.map(|c| c.floor() as i32))?;
    let (start, across) = if record.along_x { (record.corner[0], trip.entrance[0]) } else { (record.corner[2], trip.entrance[2]) };
    Some((record, (across - start as f32) / f32::from(record.span)))
}

/// Where in `exit` a traveller goes: across it as it stood in the entrance, on its bottom row;
/// without an entrance, the cell itself.
fn placed(exit: &PortalRecord, cell: [i32; 3], entrance: Option<(PortalRecord, f32)>) -> [f32; 3] {
    let Some((_, fraction)) = entrance else { return cell.map(|c| c as f32) };
    let [x, y, z] = exit.corner.map(|c| c as f32);
    let along = f32::from(exit.span) * fraction;
    if exit.along_x { [x + along, y, z + 0.5] } else { [x + 0.5, y, z + along] }
}

/// Settles the trips whose columns are loaded.
pub(crate) fn arrive(world: &mut World) {
    let due: Vec<(Entity, Trip)> = world.query::<(Entity, &Arriving)>().iter(world).filter(|(_, arriving)| !arriving.told_ready).filter_map(|(player, arriving)| Some((player, arriving.trip?))).collect();
    for (player, trip) in due {
        let (Some(layer), Some(at)) = (world.get::<InLayer>(player).map(|l| l.0), world.get::<Position>(player).map(|p| p.feet)) else { continue };
        // BDS shows the made portal to a client that holds the columns round it (bds-portal-voids.txt).
        let sent = world.get::<View>(player).is_some_and(chunks::spawn_columns_sent);
        if !loaded_round(world, layer, at) || !sent {
            continue;
        }
        settle(world, player, layer, at, trip);
    }
}

/// Whether the columns round `at` are loaded; asks for those that are not.
fn loaded_round(world: &mut World, layer: Entity, [x, _, z]: [f32; 3]) -> bool {
    let Some(mut level) = world.get_mut::<Layer>(layer) else { return false };
    let centre = ChunkPos::of_block(x.floor() as i32, z.floor() as i32);
    let mut all = true;
    for dx in -LOADED_REACH..=LOADED_REACH {
        for dz in -LOADED_REACH..=LOADED_REACH {
            all &= level.world.request_urgent(ChunkPos::new(centre.x + dx, centre.z + dz));
        }
    }
    all
}

fn settle(world: &mut World, player: Entity, layer: Entity, at: [f32; 3], trip: Trip) {
    let home = dimensions::home(world, layer);
    let Some(dimension) = world.get::<Layer>(layer).map(|level| level.world.dimension()) else { return };
    let portals = world.get::<Layer>(home).map(|level| level.world.level().portals.clone()).unwrap_or_default();
    let came_by = entrance(&portals, &trip);
    // A record whose portal is not loaded is taken at its word.
    let standing = |world: &World, (record, _): &(PortalRecord, [i32; 3])| {
        let [x, _, z] = record.corner;
        world.get::<Layer>(layer).is_some_and(|level| !level.world.is_loaded(ChunkPos::of_block(x, z)) || blocks::portal_axis(&level.world, record.corner).is_some())
    };
    let found = records::nearest(&portals, dimension, at.map(|c| c.floor() as i32)).filter(|found| standing(world, found));
    let (exit, cell, made) = match found {
        Some((record, cell)) => (record, cell, Vec::new()),
        None => {
            // BDS measures from a player's position, its eyes (the platform's y in bds-portal-voids.txt).
            let Some((record, cells)) = make(world, layer, [at[0], at[1] + EYE_HEIGHT, at[2]]) else { return };
            (record, record.corner, cells)
        }
    };
    let feet = placed(&exit, cell, came_by);
    let [yaw, pitch] = trip.look;
    if let Some(mut position) = world.get_mut::<Position>(player) {
        position.pitch = pitch;
    }
    // A portal of the other axis turns the traveller with it.
    let turn = match came_by {
        Some((entered, _)) if entered.along_x != exit.along_x => if dimension == NETHER { 90.0 } else { -90.0 },
        _ => 0.0,
    };
    movement::reset(world, player, feet, yaw + turn);
    if let Some(mut arriving) = world.get_mut::<Arriving>(player) {
        arriving.told_ready = true;
    }
    let id = world.get::<NetId>(player).map_or(0, |id| id.0);
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.push(&dimensions::ready(id));
    }
    if !made.is_empty() {
        blocks::write_shown_nether(world, layer, &made);
    }
}

/// Makes a portal for a traveller whose eyes arrive at `at`: its record and the cells to write
/// with their update flags, which [`settle`] shows after the traveller is told it is there.
fn make(world: &mut World, layer: Entity, at: [f32; 3]) -> Option<(PortalRecord, Vec<([i32; 3], u32, u32)>)> {
    let home = dimensions::home(world, layer);
    let seed = world.get::<Layer>(home)?.world.level().seed;
    let start = world.get_resource_or_insert_with(Directions::default).draw(home, seed);
    let level = &world.get::<Layer>(layer)?.world;
    let site = build::site(at, level.heights(), start, &|pos| blocks::ground(level, pos));
    let obsidian = level.registry().find("minecraft:obsidian", "")?;
    let portal = blocks::portal_id(level, site.axis())?;
    let air = level.registry().air_id();
    let mut cells: Vec<([i32; 3], u32, u32)> = Vec::new();
    if site.platform {
        let floor = blocks::NEIGHBOURS_AND_NETWORK;
        cells.extend(site.platform_cells().into_iter().map(|(pos, solid)| (pos, if solid { obsidian } else { air }, floor)));
        let netherrack = level.registry().find("minecraft:netherrack", "")?;
        let is_air = |[x, y, z]: [i32; 3]| level.in_height(y) && level.block(x, y, z) == air;
        cells.extend(site.ring_cells().into_iter().filter(|pos| is_air(*pos)).map(|pos| (pos, netherrack, floor)));
    }
    cells.extend(site.frame_cells().into_iter().map(|(pos, frame)| (pos, if frame { obsidian } else { portal }, blocks::NETWORK)));
    let shape = shape::Shape { axis: site.axis(), corner: site.corner(), width: 2, height: 3 };
    let dimension = level.dimension();
    blocks::keep(world, layer, &shape);
    Some((PortalRecord { dimension, corner: shape.corner, span: 2, along_x: shape.axis == shape::Axis::X }, cells))
}
