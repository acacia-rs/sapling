//! Where lightning rods draw a storm's bolt, as BDS's `LightningRodBlock`: to the nearest rod within
//! 64 under open sky and rain. What a strike does to a rod and copper is the blocks' (blocks/rods.rs).

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::ChunkPos;

use crate::components::Layer;

/// A rod draws a bolt from this far across and up or down.
const REACH: i32 = 64;

/// Where a bolt meant for `picked` goes instead: on top of the nearest rod within reach whose column
/// is open over it and that rain falls on.
pub(super) fn drawn_to(world: &World, layer: Entity, picked: BlockPos) -> Option<[f32; 3]> {
    let level = &world.get::<Layer>(layer)?.world;
    let [x, y, z] = picked;
    let (chunks, home) = (REACH / 16 + 1, ChunkPos::of_block(x, z));
    let columns = (-chunks..=chunks).flat_map(|dx| (-chunks..=chunks).map(move |dz| ChunkPos::new(home.x + dx, home.z + dz))).filter(|pos| level.is_loaded(*pos));
    let rods = columns.flat_map(|pos| level.find_blocks(pos, |id| level.registry().get(id).is_some_and(|state| state.name.ends_with("lightning_rod"))));
    let near = rods.filter(|[rx, ry, rz]| (rx - x).abs() <= REACH && (rz - z).abs() <= REACH && (ry - y).abs() <= REACH);
    let open = near.filter(|&[rx, ry, rz]| super::storms::top(level, rx, rz) <= ry + 1 && super::storms::rains_on(level, [rx, ry + 1, rz]));
    let far = |[rx, ry, rz]: &[i32; 3]| (rx - x).pow(2) + (ry - y).pow(2) + (rz - z).pow(2);
    open.min_by_key(far).map(|[rx, ry, rz]| [rx as f32, (ry + 1) as f32, rz as f32])
}
