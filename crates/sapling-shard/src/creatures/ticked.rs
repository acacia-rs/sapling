//! Which mobs BDS ticks: those whose column is within `tick-distance` chunks (a circle) of a player in
//! their world, or that the world keeps (kept.rs); the others neither move, think nor feel their
//! effects until a player comes near (research/captures/bds-tickcircle*.txt, docs/research/mob-damage.md).

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use sapling_world::ChunkPos;

use crate::body::Body;
use crate::components::InLayer;
use crate::kept::Kept;
use crate::view::View;

/// The players' chunks by world, taken once a tick.
pub(super) struct Ticked {
    centres: HashMap<Entity, Vec<ChunkPos>>,
    reach: i32,
}

impl Ticked {
    pub(super) fn of(world: &mut World) -> Self {
        let mut centres: HashMap<Entity, Vec<ChunkPos>> = HashMap::new();
        for (in_layer, view) in world.query::<(&InLayer, &View)>().iter(world) {
            centres.entry(in_layer.0).or_default().extend(view.center());
        }
        Self { centres, reach: crate::ticking::tick_distance(world) }
    }

    /// Whether `mob` in `layer` is ticked. A world nobody is in ticks everything: BDS's ticks
    /// nothing, but headless shards and their tests run without players.
    pub(super) fn ticks(&self, world: &World, mob: Entity, layer: Entity) -> bool {
        let Some(centres) = self.centres.get(&layer) else { return true };
        let Some(at) = world.get::<Body>(mob).map(|body| ChunkPos::of_block(body.at[0].floor() as i32, body.at[2].floor() as i32)) else { return true };
        centres.iter().any(|centre| (at.x - centre.x).pow(2) + (at.z - centre.z).pow(2) <= self.reach * self.reach) || world.get::<Kept>(layer).is_some_and(|kept| kept.holds(at))
    }
}
