//! The mobs a structure template places (an outpost's pillagers, a mansion's illagers), made as BDS
//! makes them: not with the column but in the tick after the first one in which the column ticks
//! with its eight neighbours loaded (docs/research/structure-spawns.md "When template mobs come").
//! Until then they wait with the world, kept across the column's unloads as BDS keeps them on its
//! in-memory pieces.

use std::collections::{HashMap, HashSet};

use bevy_ecs::prelude::*;
use sapling_world::ChunkPos;
use sapling_worldgen::TemplateActor;

use crate::components::Layer;
use crate::creatures;
use crate::ticking::Region;

/// A world's template mobs not made yet, by the column generation filed them under, and the
/// columns whose mob step comes next tick.
#[derive(Component, Default)]
pub(crate) struct Waiting {
    mobs: HashMap<ChunkPos, Vec<TemplateActor>>,
    due: HashSet<ChunkPos>,
}

/// Files the template mobs of a column that arrived to wait for its mob step.
pub(crate) fn arrived(world: &mut World, layer: Entity, home: ChunkPos) {
    let Some(made) = world.get_mut::<Layer>(layer).map(|mut level| level.world.take_template_actors(home)) else { return };
    if made.is_empty() {
        return;
    }
    let mut entity = world.entity_mut(layer);
    if !entity.contains::<Waiting>() {
        entity.insert(Waiting::default());
    }
    entity.get_mut::<Waiting>().expect("just inserted").mobs.entry(home).or_default().extend(made);
}

/// Makes the mobs of columns whose mob step is due and marks the columns that became ready this tick.
pub(crate) fn make_when_ready(world: &mut World) {
    let layers: Vec<Entity> = world.query_filtered::<Entity, With<Waiting>>().iter(world).collect();
    if layers.is_empty() {
        return;
    }
    let region = Region::of(world);
    for layer in layers {
        let due = world.get_mut::<Waiting>(layer).map(|mut waiting| std::mem::take(&mut waiting.due)).unwrap_or_default();
        for column in due {
            make(world, layer, column);
        }
        let Some(waiting) = world.get::<Waiting>(layer) else { continue };
        let candidates: Vec<ChunkPos> = waiting.mobs.keys().flat_map(|pos| [*pos, ChunkPos::new(pos.x - 1, pos.z), ChunkPos::new(pos.x, pos.z - 1), ChunkPos::new(pos.x - 1, pos.z - 1)]).collect();
        let ready: HashSet<ChunkPos> = candidates.into_iter().filter(|column| is_ready(world, &region, layer, *column)).collect();
        if let Some(mut waiting) = world.get_mut::<Waiting>(layer) {
            waiting.due = ready;
        }
    }
}

/// Whether `column` ticks with its eight neighbours loaded.
fn is_ready(world: &World, region: &Region, layer: Entity, column: ChunkPos) -> bool {
    let Some(level) = world.get::<Layer>(layer) else { return false };
    let loaded = (-1..=1).all(|dx| (-1..=1).all(|dz| level.world.is_loaded(ChunkPos::new(column.x + dx, column.z + dz))));
    loaded && region.ticks(world, layer, column)
}

/// The mob step of `column`: the waiting mobs in its box, which reaches a block into the columns
/// east and south (BDS's box is x0..x0+16 and z0..z0+16); persistent as any stored mob. With
/// `doMobSpawning` off BDS skips the step and the mobs never come (bds-t12actors.txt).
fn make(world: &mut World, layer: Entity, column: ChunkPos) {
    let (x0, z0) = (column.x * 16, column.z * 16);
    let inside = |actor: &TemplateActor| {
        let (x, z) = (actor.feet[0].floor() as i32, actor.feet[2].floor() as i32);
        (x0..=x0 + 16).contains(&x) && (z0..=z0 + 16).contains(&z)
    };
    let Some(mut waiting) = world.get_mut::<Waiting>(layer) else { return };
    let mut made = Vec::new();
    for home in [column, ChunkPos::new(column.x + 1, column.z), ChunkPos::new(column.x, column.z + 1), ChunkPos::new(column.x + 1, column.z + 1)] {
        let Some(mobs) = waiting.mobs.get_mut(&home) else { continue };
        let (taken, kept): (Vec<_>, Vec<_>) = mobs.drain(..).partition(|actor| inside(actor));
        made.extend(taken);
        if kept.is_empty() {
            waiting.mobs.remove(&home);
        } else {
            *waiting.mobs.get_mut(&home).expect("still there") = kept;
        }
    }
    if made.is_empty() || !crate::rules::on(world, layer, "doMobSpawning") {
        return;
    }
    for actor in made {
        let event = Some(actor.event).filter(|event| *event != "minecraft:entity_spawned");
        creatures::spawn_as(world, layer, actor.kind, actor.feet, sapling_api::SpawnReason::Structure, 0.0, event);
    }
}
