//! Which mobs BDS ticks: those whose column ticks (`crate::ticking::Region`: a player's tick region in
//! their world, or a kept column); the others neither move, think nor feel their effects until a player
//! comes near (research/captures/bds-tickcircle*.txt, docs/research/mob-damage.md).

use bevy_ecs::prelude::*;
use sapling_world::ChunkPos;

use crate::body::Body;
use crate::ticking::Region;

/// The tick's region, taken once a tick.
pub(super) struct Ticked(Region);

impl Ticked {
    pub(super) fn of(world: &mut World) -> Self {
        Self(Region::of(world))
    }

    /// Whether `mob` in `layer` is ticked ([`Region::ticks`]).
    pub(super) fn ticks(&self, world: &World, mob: Entity, layer: Entity) -> bool {
        let Some(at) = world.get::<Body>(mob).map(|body| ChunkPos::of_block(body.at[0].floor() as i32, body.at[2].floor() as i32)) else { return true };
        self.0.ticks(world, layer, at)
    }
}
