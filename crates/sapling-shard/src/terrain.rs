//! A shard's world as acacia-physics reads it: players' steps (simulation.rs) and bodies' moves (body.rs).

use acacia_physics::registry_world::{RegistryWorld, Terrain};
use acacia_physics::{Aabb, BlockPhysics, WorldView};
use acacia_world::BlockAccess;
use sapling_api::BlockPos;
use sapling_world::{ChunkPos, World as Level};

/// The blocks of a level, with `cleared` cells read as air.
pub(crate) struct Blocks<'a>(pub &'a Level, pub &'a [BlockPos]);

impl BlockAccess for Blocks<'_> {
    fn block(&self, x: i32, y: i32, z: i32) -> u32 {
        crate::blocks::custom::stand_in(self.0.registry(), self.raw(x, y, z))
    }

    fn liquid(&self, x: i32, y: i32, z: i32) -> u32 {
        self.0.liquid(x, y, z)
    }
}

impl Blocks<'_> {
    fn raw(&self, x: i32, y: i32, z: i32) -> u32 {
        if self.1.contains(&[x, y, z]) { self.0.registry().air_id() } else { self.0.block(x, y, z) }
    }
}

impl Terrain for Blocks<'_> {
    fn is_column_loaded(&self, cx: i32, cz: i32) -> bool {
        self.0.is_loaded(ChunkPos::new(cx, cz))
    }
}

/// The physics' view: vanilla blocks by the registry's boxes, a custom block by the collision box
/// its plugin gave it (its permutations' boxes are not read: the client evaluates their conditions).
pub(crate) struct ShardView<'a, 'b>(RegistryWorld<'a, Blocks<'b>>);

impl WorldView for ShardView<'_, '_> {
    fn block_collisions(&self, pos: BlockPos, out: &mut Vec<Aabb>) {
        let [x, y, z] = pos;
        match crate::blocks::custom::collision_box(self.0.view.raw(x, y, z)) {
            Some(found) => out.extend(found),
            None => self.0.block_collisions(pos, out),
        }
    }

    fn block(&self, pos: BlockPos) -> BlockPhysics {
        self.0.block(pos)
    }

    fn is_area_loaded(&self, area: &Aabb) -> bool {
        self.0.is_area_loaded(area)
    }
}

/// Runs `f` on the physics' view of `level`.
pub(crate) fn physics_view<R>(level: &Level, f: impl FnOnce(&ShardView<'_, '_>) -> R) -> R {
    physics_view_clearing(level, &[], f)
}

/// [`physics_view`] with the blocks of `cleared` read as air.
pub(crate) fn physics_view_clearing<R>(level: &Level, cleared: &[BlockPos], f: impl FnOnce(&ShardView<'_, '_>) -> R) -> R {
    let terrain = Blocks(level, cleared);
    f(&ShardView(RegistryWorld { view: &terrain, registry: level.registry() }))
}
