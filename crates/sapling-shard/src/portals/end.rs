//! Travel by end portal, as BDS made it (research/captures/bds-end-frame.txt): a player whose body
//! is in an end portal block goes to the End at once, is corrected there unmoving, put on the
//! arrival place by MovePlayer Reset once the columns round it are loaded and told it is ready;
//! the obsidian platform under it is built the tick after, in the cells that differ.

use bevy_ecs::prelude::*;
use sapling_world::{ChunkPos, END};

use crate::blocks::portals::write_shown_flagged;
use crate::components::{InLayer, Layer, NetId, Outgoing, Position};
use crate::dimensions::{self, Arriving};
use crate::movement;

/// Where BDS puts a player arriving by end portal: feet, the cell's corner.
pub(crate) const ARRIVAL: [f32; 3] = [100.0, 50.0, 0.0];
/// Columns this far round the arrival must be loaded before it is settled.
const LOADED_REACH: i32 = 2;
/// The flags BDS sent the platform's cells with.
const NEIGHBOURS_AND_NETWORK: u32 = 3;

/// Platforms to build next tick, by End layer.
#[derive(Resource, Default)]
pub(crate) struct Platforms(Vec<Entity>);

/// Sends a player from its world to the End of its realm.
pub(super) fn depart(world: &mut World, player: Entity) {
    let Some(layer) = world.get::<InLayer>(player).map(|in_layer| in_layer.0) else { return };
    let Some(end) = dimensions::of(world, layer, END).filter(|end| *end != layer) else { return };
    dimensions::travel(world, player, end, ARRIVAL);
    movement::correct_unmoving(world, player, ARRIVAL);
    if let Some(mut arriving) = world.get_mut::<Arriving>(player) {
        arriving.to_platform = true;
    }
}

/// Builds the platforms of last tick's arrivals, then puts this tick's arrivals whose columns are
/// loaded in place.
pub(crate) fn arrive(world: &mut World) {
    let built = world.get_resource_mut::<Platforms>().map(|mut platforms| std::mem::take(&mut platforms.0)).unwrap_or_default();
    for layer in built {
        build_platform(world, layer);
    }
    let due: Vec<Entity> = world.query::<(Entity, &Arriving)>().iter(world).filter(|(_, arriving)| arriving.to_platform && !arriving.told_ready).map(|(player, _)| player).collect();
    for player in due {
        let Some(layer) = world.get::<InLayer>(player).map(|in_layer| in_layer.0) else { continue };
        if !loaded_round(world, layer) {
            continue;
        }
        // BDS's `moveTo` keeps the yaw and levels the pitch.
        let Some(mut position) = world.get_mut::<Position>(player) else { continue };
        position.pitch = 0.0;
        let yaw = position.yaw;
        movement::reset(world, player, ARRIVAL, yaw);
        if let Some(mut arriving) = world.get_mut::<Arriving>(player) {
            arriving.told_ready = true;
        }
        let id = world.get::<NetId>(player).map_or(0, |id| id.0);
        if let Some(mut out) = world.get_mut::<Outgoing>(player) {
            out.push(&dimensions::ready(id));
        }
        world.get_resource_or_insert_with(Platforms::default).0.push(layer);
    }
}

/// Whether the columns round the arrival are loaded; asks for those that are not.
fn loaded_round(world: &mut World, layer: Entity) -> bool {
    let Some(mut level) = world.get_mut::<Layer>(layer) else { return false };
    let centre = ChunkPos::of_block(ARRIVAL[0] as i32, ARRIVAL[2] as i32);
    let mut all = true;
    for dx in -LOADED_REACH..=LOADED_REACH {
        for dz in -LOADED_REACH..=LOADED_REACH {
            all &= level.world.request_urgent(ChunkPos::new(centre.x + dx, centre.z + dz));
        }
    }
    all
}

/// The platform's cells in BDS's order: z from 2 down to -2, x up, then y: obsidian at 49 under
/// the arrival, air in the three above.
pub(crate) fn platform_cells() -> Vec<([i32; 3], bool)> {
    let [x, y, z] = ARRIVAL.map(|c| c as i32);
    (-2..=2).rev().flat_map(|dz| (-2..=2).flat_map(move |dx| (-1..=2).map(move |dy| ([x + dx, y + dy, z + dz], dy == -1)))).collect()
}

fn build_platform(world: &mut World, layer: Entity) {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let (Some(obsidian), air) = (level.registry().find("minecraft:obsidian", ""), level.registry().air_id()) else { return };
    let cells: Vec<([i32; 3], u32)> = platform_cells().into_iter().map(|(pos, solid)| (pos, if solid { obsidian } else { air })).collect();
    write_shown_flagged(world, layer, &cells, NEIGHBOURS_AND_NETWORK);
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_platform_is_five_by_five_at_49_with_three_of_air_over_it() {
        let cells = super::platform_cells();
        assert_eq!(cells.len(), 100);
        assert_eq!(&cells[..5], &[([98, 49, 2], true), ([98, 50, 2], false), ([98, 51, 2], false), ([98, 52, 2], false), ([99, 49, 2], true)]);
        assert_eq!(cells.last(), Some(&([102, 52, -2], false)));
    }
}
