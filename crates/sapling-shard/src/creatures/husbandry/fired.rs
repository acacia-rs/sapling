//! A raised mob's pack events fired by `/event entity` that change what sapling keeps of it
//! (research/captures/bds-bc1event.txt): a baby's `minecraft:ageable_grow_up`, a sheep's wool
//! (`minecraft:on_sheared`, `minecraft:on_eat_block`) and colour (`wololo` and the three
//! `*_color` rolls of its `minecraft:entity_spawned`), and a sniffer's `on_scenting_success`.

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
        "on_scenting_success" => {
            let now = world.resource::<crate::components::Tick>().0;
            super::sniffing::scented(world, entity, kind, now);
        }
        _ => {
            let Some((_, weights)) = COLOURS.iter().find(|(name, _)| *name == event) else { return };
            let total = weights.iter().map(|(weight, _)| weight).sum();
            let mut drawn = world.get_resource_or_insert_with(Dice::default).roll(total);
            let colour = weights.iter().find_map(|&(weight, colour)| (drawn < weight).then_some(colour).or_else(|| { drawn -= weight; None })).unwrap_or(0);
            products::colour(world, entity, colour);
        }
    }
}
