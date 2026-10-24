//! Mobs that swim up in water, as BDS's with `minecraft:behavior.float` (research/captures/bds-swim.txt):
//! with water more than [`DEPTH`] over its feet, a mob swims up [`UP`] faster, after the drag and
//! the pull down of sinking, on four ticks in five; a zombie, a skeleton or a golem sinks.
// TODO: how deep BDS lets it be before it swims is Java's FloatGoal threshold, not fitted.

use bevy_ecs::prelude::*;
use sapling_world::World as Level;

use crate::blocks::Dice;
use crate::body::{self, Body, Liquid};
use crate::components::Layer;

const DEPTH: f32 = 0.4;
const UP: f32 = 0.04;
const CHANCE: f32 = 0.8;

/// How far the surface of the water the body stands in is over its feet; 0 out of water.
fn depth(level: &Level, body: &Body) -> f32 {
    let [x, feet, z] = body.at;
    let (x, z) = (x.floor() as i32, z.floor() as i32);
    let mut y = feet.floor() as i32;
    let Some(mut found) = body::liquid_in(level, [x, y, z], Liquid::Water) else { return 0.0 };
    while let Some(above) = body::liquid_in(level, [x, y + 1, z], Liquid::Water) {
        (y, found) = (y + 1, above);
    }
    body::surface(y, found) - feet
}

/// Before its move: a mob of `kind` deep in water may swim up this tick.
pub(super) fn tick(world: &mut World, entity: Entity, layer: Entity, kind: &str, start: &Body) {
    if !sapling_data::mob_traits(kind).floats || !world.get::<Layer>(layer).is_some_and(|layer| depth(&layer.world, start) > DEPTH) {
        return;
    }
    if world.get_resource_or_insert_with(Dice::default).fraction() < CHANCE
        && let Some(mut body) = world.get_mut::<Body>(entity)
    {
        body.swim_up(UP);
    }
}
