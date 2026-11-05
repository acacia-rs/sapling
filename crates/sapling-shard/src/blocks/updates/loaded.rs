//! What BDS did to a structure load's cells once all were in (docs/research/structure-blocks.md, "After a
//! load"; research/captures/bds-e3turn.txt): fences and walls join what is there, glow lichen and resin
//! drop the faces with nothing solid behind, and torches, activator rails, cocoa and hanging signs that lack
//! support show breaking and go a tick later with their items.

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};

use super::super::{level, send, shapes, with_property};
use super::{lacks_support, loosened};

/// What BDS's load took out unheld; levers, plain and golden rails, comparators, petals, leaf litter and
/// big dripleaves it kept wherever `lacks_support` would refuse them (bds-e3turn.txt).
const POPS: [&str; 4] = ["torch", ":activator_rail", ":cocoa", "_hanging_sign"];
const CLINGS: [&str; 2] = ["minecraft:glow_lichen", "minecraft:resin_clump"];
/// Each `multi_face_direction_bits` bit's face, as facing.rs places them.
const BITS: [(Face, u32); 6] = [(Face::Down, 1), (Face::Up, 2), (Face::South, 4), (Face::West, 8), (Face::North, 16), (Face::East, 32)];

/// Lets the blocks just loaded at `cells` react to their neighbours.
pub(crate) fn settle(world: &mut World, layer: Entity, cells: &[BlockPos]) {
    for &pos in cells {
        shapes::reshape_around(world, layer, pos);
    }
    for &pos in cells {
        let Some(level) = level(world, layer).filter(|level| level.in_height(pos[1])) else { continue };
        let id = level.block(pos[0], pos[1], pos[2]);
        match held_faces(level, pos, id) {
            Some((bits, kept)) if kept == bits => {}
            Some((_, 0)) => loosened::noticed(world, layer, pos),
            Some((_, kept)) => {
                if let Some(id) = with_property(level.registry(), id, "multi_face_direction_bits", &kept.to_string()) {
                    send::set(world, layer, pos, id);
                }
            }
            None if level.registry().get(id).is_some_and(|state| POPS.iter().any(|end| state.name.ends_with(end))) && lacks_support(world, layer, pos, id) => {
                loosened::noticed(world, layer, pos)
            }
            None => {}
        }
    }
}

/// The faces of the clinging block `id` at `pos` and those of them with something solid behind.
fn held_faces(level: &sapling_world::World, pos: BlockPos, id: u32) -> Option<(u32, u32)> {
    let registry = level.registry();
    let state = registry.get(id).filter(|state| CLINGS.contains(&state.name))?;
    let bits: u32 = state.properties.split(',').find_map(|pair| pair.strip_prefix("multi_face_direction_bits="))?.parse().ok()?;
    let behind_is_solid = |face: Face| {
        let [x, y, z] = face.neighbour(pos);
        level.in_height(y) && level.is_loaded(send::column([x, y, z])) && registry.get(level.block(x, y, z)).is_some_and(|state| state.is_solid())
    };
    Some((bits, BITS.iter().filter(|&&(face, bit)| bits & bit != 0 && behind_is_solid(face)).fold(0, |kept, (_, bit)| kept | bit)))
}
