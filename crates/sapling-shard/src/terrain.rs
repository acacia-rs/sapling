//! A shard's world as acacia-physics reads it: players' steps (simulation.rs) and bodies' moves (body.rs).

use acacia_physics::registry_world::{RegistryWorld, Terrain};
use acacia_world::BlockAccess;
use sapling_world::{ChunkPos, World as Level};

pub(crate) struct Blocks<'a>(pub &'a Level);

impl BlockAccess for Blocks<'_> {
    fn block(&self, x: i32, y: i32, z: i32) -> u32 {
        self.0.block(x, y, z)
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
    let terrain = Blocks(level);
    f(&RegistryWorld { view: &terrain, registry: level.registry() })
}
