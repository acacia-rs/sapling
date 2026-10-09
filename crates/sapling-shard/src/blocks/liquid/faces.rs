//! The faces water may pass through: BDS `liquidCanFlowIntoFromDirection`, which only stairs,
//! slabs, sculk sensors, heavy cores and shelves answer for themselves; every other block is open
//! on all six (docs/research/liquid-faces.md). Water moves between two cells only through two
//! open faces, the one it leaves and the one it enters, so a waterlogged block keeps its water in
//! where it is closed.

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};
use sapling_world::BlockRegistry;

use super::super::{level, send};

pub(super) fn opposite(face: Face) -> Face {
    match face {
        Face::Down => Face::Up,
        Face::Up => Face::Down,
        Face::North => Face::South,
        Face::South => Face::North,
        Face::West => Face::East,
        Face::East => Face::West,
    }
}

fn property<'a>(registry: &'a BlockRegistry, id: u32, key: &str) -> Option<&'a str> {
    registry.get(id)?.property(key)
}

/// A stair's closed back, by its `weirdo_direction`: 0 east, 1 west, 2 south, 3 north.
const STAIR_BACKS: [Face; 4] = [Face::East, Face::West, Face::South, Face::North];

/// Whether the block at `pos` lets water through its `face`.
pub(super) fn open(world: &World, layer: Entity, pos: BlockPos, face: Face) -> bool {
    let Some(level) = level(world, layer).filter(|level| level.in_height(pos[1]) && level.is_loaded(send::column(pos))) else { return true };
    let (registry, id) = (level.registry(), level.block(pos[0], pos[1], pos[2]));
    let Some(name) = registry.get(id).map(|state| state.name) else { return true };
    if name.ends_with("_stairs") {
        let top = property(registry, id, "upside_down_bit") == Some("1");
        let Some(back) = property(registry, id, "weirdo_direction").and_then(|d| d.parse::<usize>().ok()).and_then(|d| STAIR_BACKS.get(d).copied()) else { return true };
        return match face {
            Face::Down => top,
            Face::Up => !top,
            _ if face == back => false,
            _ if face == opposite(back) => true,
            // A side is closed by an inner corner: the stair in front turned with its back that way.
            side => {
                let front = opposite(back).neighbour(pos);
                let turned = STAIR_BACKS.iter().position(|back| *back == side).map(|d| d.to_string());
                let corner = registry.get(level.block(front[0], front[1], front[2])).is_some_and(|state| state.name.ends_with("_stairs") && state.property("weirdo_direction") == turned.as_deref());
                !corner
            }
        };
    }
    if name.ends_with("_slab") && !name.ends_with("double_slab") {
        let top = property(registry, id, "minecraft:vertical_half") == Some("top");
        return !matches!((face, top), (Face::Down, false) | (Face::Up, true));
    }
    if name.ends_with("sculk_sensor") || name == "minecraft:heavy_core" {
        return face != Face::Down;
    }
    if name.ends_with("_shelf") {
        let facing = match property(registry, id, "minecraft:cardinal_direction") {
            Some("north") => Face::North,
            Some("south") => Face::South,
            Some("west") => Face::West,
            Some("east") => Face::East,
            _ => return true,
        };
        return face != opposite(facing);
    }
    true
}

/// Whether water at `from` may move to its neighbour on `toward`: out through that face, in
/// through the neighbour's opposite one.
pub(super) fn passes(world: &World, layer: Entity, from: BlockPos, toward: Face) -> bool {
    open(world, layer, from, toward) && open(world, layer, toward.neighbour(from), opposite(toward))
}
