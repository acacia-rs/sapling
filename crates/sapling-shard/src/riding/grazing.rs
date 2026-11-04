//! A horse-family mob grazing: bit 32 (`HorseFlags::Eating`) of its data 16, told with its mouth
//! (mouth.rs). BDS's `Horse::postAiStep` (0x142a49530): a horse eating nothing and carrying no one
//! starts one tick in 300, if its feet are on a grass block, and stops 50 ticks on or in the first
//! tick it has a path to walk.

use bevy_ecs::prelude::*;

use super::Seats;
use crate::blocks::Dice;
use crate::components::Layer;

pub(super) const EATING: i64 = 32;
const CHANCE: u32 = 300;
const EATS: u32 = 50;

/// How many ticks it has eaten, while it eats.
#[derive(Component, Default)]
pub(super) struct Grazing(Option<u32>);

/// Grazing's own draws, so that a horse's tick shifts no other roll.
#[derive(Resource, Default)]
struct Bites(Dice);

fn on_grass(world: &World, layer: Entity, feet: [f32; 3]) -> bool {
    let [x, y, z] = feet.map(|c| c.floor() as i32);
    world.get::<Layer>(layer).is_some_and(|layer| crate::creatures::holds(&layer.world, [x, y - 1, z], &["minecraft:grass_block"]))
}

/// A tick of the horse `entity`'s grazing: whether it eats now, and whether it started and
/// stopped in this one tick (it set off on a stroll), which BDS still tells as a change.
pub(super) fn graze(world: &mut World, entity: Entity, layer: Entity, feet: [f32; 3]) -> (bool, bool) {
    let carries = world.get::<Seats>(entity).is_some_and(|seats| !seats.0.is_empty());
    let mut eaten = world.get::<Grazing>(entity).and_then(|grazing| grazing.0);
    let mut began = false;
    if eaten.is_none() && !carries && world.get_resource_or_insert_with(Bites::default).0.roll(CHANCE) == 0 && on_grass(world, layer, feet) {
        (eaten, began) = (Some(0), true);
    }
    let walks = crate::creatures::is_under_way(world, entity);
    let eaten = eaten.filter(|ticks| !walks && *ticks < EATS).map(|ticks| ticks + 1);
    world.entity_mut(entity).insert(Grazing(eaten));
    (eaten.is_some(), began && eaten.is_none())
}
