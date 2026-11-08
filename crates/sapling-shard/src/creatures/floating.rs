//! Mobs that swim up in water, as BDS's with `minecraft:behavior.float` (research/captures/bds-swim.txt):
//! with water more than [`DEPTH`] over its feet, a mob swims up [`UP`] faster, after the drag and
//! the pull down of sinking, on four ticks in five; a zombie, a skeleton or a golem sinks. A slime
//! or magma cube (`behavior.slime_float`) does so in lava too, at any depth: BDS's small slimes rose
//! 0.02, 0.03, 0.035 ... a tick, lava's halving and 0.02 pull under the same push, missing one tick
//! in five, up to its top (research/captures/bds-j3slime.txt).
// DEPTH is Java's FloatGoal threshold; drilled (bds-n3float.txt): in flowing water of levels 1 to 6
// neither server's cows swam up, and chickens only where their box reached the source.

use bevy_ecs::prelude::*;
use sapling_world::World as Level;

use crate::blocks::Dice;
use crate::body::{self, Body, Liquid};
use crate::components::Layer;

const DEPTH: f32 = 0.4;
const UP: f32 = 0.04;
const CHANCE: f32 = 0.8;
const IN_LAVA: &[&str] = &["minecraft:slime", "minecraft:magma_cube"];

/// How far the surface of the `liquid` the body stands in is over its feet; 0 out of it.
fn depth(level: &Level, body: &Body, liquid: Liquid) -> f32 {
    let [x, feet, z] = body.at;
    let (x, z) = (x.floor() as i32, z.floor() as i32);
    let mut y = feet.floor() as i32;
    let Some(mut found) = body::liquid_in(level, [x, y, z], liquid) else { return 0.0 };
    while let Some(above) = body::liquid_in(level, [x, y + 1, z], liquid) {
        (y, found) = (y + 1, above);
    }
    body::surface(y, found) - feet
}

/// Before its move: a mob of `kind` deep in water, or a slime in lava, may swim up this tick.
pub(super) fn tick(world: &mut World, entity: Entity, layer: Entity, kind: &str, start: &Body) {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let floats = sapling_data::mob_traits(kind).floats && depth(level, start, Liquid::Water) > DEPTH;
    let in_lava = IN_LAVA.contains(&kind) && depth(level, start, Liquid::Lava) > 0.0;
    if (floats || in_lava)
        && world.get_resource_or_insert_with(Dice::default).fraction() < CHANCE
        && let Some(mut body) = world.get_mut::<Body>(entity)
    {
        body.swim_up(UP);
    }
}
