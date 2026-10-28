//! A shard's world as acacia-physics reads it: players' steps (simulation.rs) and bodies' moves (body.rs).

use acacia_physics::registry_world::{RegistryWorld, Terrain};
use acacia_world::BlockAccess;
use sapling_api::BlockPos;
use sapling_world::{ChunkPos, World as Level};

/// The blocks of a level, with `cleared` cells read as air.
pub(crate) struct Blocks<'a>(pub &'a Level, pub &'a [BlockPos]);

impl BlockAccess for Blocks<'_> {
    fn block(&self, x: i32, y: i32, z: i32) -> u32 {
        if self.1.contains(&[x, y, z]) { self.0.registry().air_id() } else { crate::blocks::custom::stand_in(self.0.registry(), self.0.block(x, y, z)) }
    }

    fn liquid(&self, x: i32, y: i32, z: i32) -> u32 {
        self.0.liquid(x, y, z)
    }
}

impl Terrain for Blocks<'_> {
    fn is_column_loaded(&self, cx: i32, cz: i32) -> bool {
        self.0.is_loaded(ChunkPos::new(cx, cz))
    }
}

/// Runs `f` on the physics' view of `level`.
pub(crate) fn physics_view<R>(level: &Level, f: impl FnOnce(&RegistryWorld<'_, Blocks<'_>>) -> R) -> R {
    physics_view_clearing(level, &[], f)
}

/// [`physics_view`] with the blocks of `cleared` read as air.
pub(crate) fn physics_view_clearing<R>(level: &Level, cleared: &[BlockPos], f: impl FnOnce(&RegistryWorld<'_, Blocks<'_>>) -> R) -> R {
    let terrain = Blocks(level, cleared);
    f(&RegistryWorld { view: &terrain, registry: level.registry() })
}
