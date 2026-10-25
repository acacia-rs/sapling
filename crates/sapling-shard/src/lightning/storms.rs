//! Where a thunderstorm strikes, as BDS's `LevelChunk::tick`: each column near a player has one
//! chance in 100000 a tick of a bolt, at a block picked in it on the top of what stands there; a
//! lightning rod near draws it (rods.rs), else an animal of the column under open sky at random,
//! and it comes only where rain falls.
// TODO: BDS's skeleton horse trap is not set; the animals are the kinds of its `Animal` type as
// read from its actor type ids, not checked one by one.

use bevy_ecs::prelude::*;
use sapling_world::{ChunkPos, FULL_LIGHT};

use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Layer, in_world};
use crate::entities::Kind;
use crate::view::View;

const ONE_IN: u32 = 100_000;
/// Columns this many from a player's tick.
const NEAR: i32 = 4;
const ANIMALS: &[&str] = &[
    "minecraft:chicken", "minecraft:cow", "minecraft:pig", "minecraft:sheep", "minecraft:wolf", "minecraft:mooshroom", "minecraft:rabbit", "minecraft:polar_bear", "minecraft:llama",
    "minecraft:turtle", "minecraft:panda", "minecraft:fox", "minecraft:goat",
];

/// The columns of `layer` near its players.
fn columns(world: &World, layer: Entity) -> Vec<ChunkPos> {
    let mut near: Vec<ChunkPos> = Vec::new();
    for player in in_world(world) {
        let (Some(on), Some(view)) = (world.get::<InLayer>(player), world.get::<View>(player)) else { continue };
        let Some(centre) = view.center().filter(|_| on.0 == layer) else { continue };
        let reach = NEAR.min(view.radius());
        near.extend((-reach..=reach).flat_map(|x| (-reach..=reach).map(move |z| ChunkPos::new(centre.x + x, centre.z + z))));
    }
    near.sort_unstable_by_key(|pos| (pos.x, pos.z));
    near.dedup();
    let level = world.get::<Layer>(layer).map(|layer| &layer.world);
    near.retain(|pos| level.is_some_and(|level| level.is_loaded(*pos)));
    near
}

/// Whether rain falls on the cell `[x, y, z]`: the sky is over it and it is not cold enough for snow.
pub(super) fn rains_on(level: &sapling_world::World, [x, y, z]: [i32; 3]) -> bool {
    level.light_at(x, y, z).is_some_and(|(sky, _)| sky == FULL_LIGHT) && !level.is_cold(x, y, z)
}

/// The cell over the top of what stands in the column `(x, z)`.
pub(super) fn top(level: &sapling_world::World, x: i32, z: i32) -> i32 {
    level.surface(x, z).map_or(level.heights().start, |top| top + 1)
}

/// The feet of an animal at random of the column `column` that stands under open sky.
fn animal(world: &mut World, layer: Entity, column: ChunkPos) -> Option<[f32; 3]> {
    let mut bodies = world.query::<(&InLayer, &Body, &Kind)>();
    let level = &world.get::<Layer>(layer)?.world;
    let found: Vec<[f32; 3]> = bodies
        .iter(world)
        .filter(|(on, body, kind)| on.0 == layer && ChunkPos::of_block(body.at[0].floor() as i32, body.at[2].floor() as i32) == column && ANIMALS.contains(&kind.0.as_str()))
        .map(|(_, body, _)| body.at)
        .filter(|feet| feet[1].floor() as i32 >= top(level, feet[0].floor() as i32, feet[2].floor() as i32))
        .collect();
    (!found.is_empty()).then(|| found[world.get_resource_or_insert_with(Dice::default).roll(found.len() as u32) as usize])
}

/// Strikes where the dice say in each world that thunders.
pub(super) fn tick(world: &mut World) {
    let layers: Vec<Entity> = world.query_filtered::<Entity, With<Layer>>().iter(world).collect();
    let stormy: Vec<Entity> = layers.into_iter().filter(|layer| super::thunders(world, *layer)).collect();
    for layer in stormy {
        for column in columns(world, layer) {
            let (struck, dx, dz) = {
                let mut dice = world.get_resource_or_insert_with(Dice::default);
                (dice.roll(ONE_IN) == 0, dice.roll(16) as i32, dice.roll(16) as i32)
            };
            let (x, z) = (column.x * 16 + dx, column.z * 16 + dz);
            let Some(y) = world.get::<Layer>(layer).map(|layer| top(&layer.world, x, z)).filter(|_| struck) else { continue };
            let picked = [x, y, z];
            let at = super::rods::drawn_to(world, layer, picked).or_else(|| animal(world, layer, column)).unwrap_or(picked.map(|c| c as f32));
            if world.get::<Layer>(layer).is_some_and(|layer| rains_on(&layer.world, at.map(|c| c.floor() as i32))) {
                super::strike(world, layer, at, true);
            }
        }
    }
}
