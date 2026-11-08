//! A raised mob's pack events fired by `/event entity` that change what sapling keeps of it
//! (research/captures/bds-bc1event.txt): a baby's `minecraft:ageable_grow_up`, a sheep's wool
//! (`minecraft:on_sheared`, `minecraft:on_eat_block`) and colour (`wololo` and the three
//! `*_color` rolls of its `minecraft:entity_spawned`).

use bevy_ecs::prelude::*;

use super::{ageing, products};
use crate::blocks::Dice;

/// Each colour event's weights over the wool colours (`Color`) it adds, as `sheep.json` has them.
const COLOURS: [(&str, [(u32, i8); 6]); 3] = [
    ("minecraft:temperate_color", [(81836, 0), (5000, 15), (5000, 8), (5000, 7), (3000, 12), (164, 6)]),
    ("minecraft:cold_color", [(81836, 15), (5000, 8), (5000, 7), (5000, 0), (3000, 12), (164, 6)]),
    ("minecraft:warm_color", [(81836, 12), (5000, 7), (5000, 8), (5000, 0), (3000, 15), (164, 6)]),
];
/// The evoker's `wololo` turns a sheep red.
const RED: i8 = 14;

/// Does what `event` (one `kind`'s definition has) changes of a raised mob; nothing for the rest.
pub(in crate::creatures) fn fired(world: &mut World, entity: Entity, kind: &str, event: &str) {
    match event {
        "minecraft:ageable_grow_up" => ageing::grow_up_by_event(world, entity, kind),
        "minecraft:on_sheared" => products::shorn(world, entity, kind, true),
        "minecraft:on_eat_block" => products::shorn(world, entity, kind, false),
        "wololo" => products::colour(world, entity, RED),
        _ => {
            if let Some(colour) = rolled_colour(world, event) {
                products::colour(world, entity, colour);
            }
        }
    }
}

/// The colour a newly spawned sheep's `minecraft:entity_spawned` rolls, by its biome's farm-animal
/// `climate` (none for temperate); None for any other kind.
pub(super) fn spawned_colour(world: &mut World, kind: &str, climate: Option<&str>) -> Option<i8> {
    (kind == "minecraft:sheep").then_some(())?;
    rolled_colour(world, &format!("minecraft:{}_color", climate.unwrap_or("temperate")))
}

fn rolled_colour(world: &mut World, event: &str) -> Option<i8> {
    let (_, weights) = COLOURS.iter().find(|(name, _)| *name == event)?;
    let total = weights.iter().map(|(weight, _)| weight).sum();
    let mut drawn = world.get_resource_or_insert_with(Dice::default).roll(total);
    Some(weights.iter().find_map(|&(weight, colour)| (drawn < weight).then_some(colour).or_else(|| { drawn -= weight; None })).unwrap_or(0))
}
