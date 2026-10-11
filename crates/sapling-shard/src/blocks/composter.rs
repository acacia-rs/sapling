//! Composters: what rots goes in by chance a click, and a full one gives bone meal
//! (docs/research/block-uses.md, "Composter").
// TODO: hoppers neither feed nor empty one; a composter filled to 7 is ready at once, where
// the Java game waits a second; emptying one was not measured on BDS.

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Item};

use super::{Dice, level, send, with_property};
use crate::{drops, inventory};

pub(super) const COMPOSTER: &str = "minecraft:composter";
const LEVEL: &str = "composter_fill_level";
const FULL: u8 = 7;
const READY: u8 = 8;
/// Where BDS put the particles over the cell's corner.
const PARTICLES_UP: f32 = 0.75;

/// The chance that `item` raises a composter: the Java game's table by the end of the name, of
/// which BDS showed wheat seeds raise one in two clicks of six.
const ROTS: &[(&str, f32)] = &[
    ("_seeds", 0.3), ("_leaves", 0.3), ("_sapling", 0.3), (":short_grass", 0.3), (":kelp", 0.3), ("_berries", 0.3), (":seagrass", 0.3), (":moss_carpet", 0.3),
    (":cactus", 0.5), (":melon_slice", 0.5), (":sugar_cane", 0.5), (":vine", 0.5), (":tall_grass", 0.5), (":nether_sprouts", 0.5),
    (":apple", 0.65), (":carrot", 0.65), (":potato", 0.65), (":wheat", 0.65), (":beetroot", 0.65), (":cocoa_beans", 0.65), (":waterlily", 0.65), (":pumpkin", 0.65),
    (":melon_block", 0.65), ("_mushroom", 0.65), (":poppy", 0.65), (":dandelion", 0.65), ("_tulip", 0.65), (":fern", 0.65), (":sea_pickle", 0.65), (":moss_block", 0.65),
    (":bread", 0.85), (":baked_potato", 0.85), (":cookie", 0.85), (":hay_block", 0.85), ("_wart_block", 0.85),
    (":cake", 1.0), (":pumpkin_pie", 1.0),
];

fn chance(item: &str) -> Option<f32> {
    ROTS.iter().find(|(end, _)| item.ends_with(end)).map(|(_, chance)| *chance)
}

fn filled(world: &World, layer: Entity, [x, y, z]: BlockPos) -> Option<(u32, u8)> {
    let level = level(world, layer)?;
    let id = level.block(x, y, z);
    let state = level.registry().get(id).filter(|state| state.name == COMPOSTER)?;
    let filled = state.properties.split(',').filter_map(|pair| pair.split_once('=')).find(|(key, _)| *key == LEVEL)?.1.parse().ok()?;
    Some((id, filled))
}

fn set_level(world: &mut World, layer: Entity, pos: BlockPos, id: u32, to: u8) {
    let now = level(world, layer).and_then(|level| with_property(level.registry(), id, LEVEL, &to.to_string()));
    if let Some(now) = now {
        send::set(world, layer, pos, now);
    }
}

fn hear(world: &mut World, layer: Entity, pos: BlockPos, name: &str) {
    let [x, y, z] = pos.map(|c| c as f32 + 0.5);
    send::to_viewers(world, layer, pos, &[send::noise(name, Vec3f { x, y, z }, -1)]);
}

/// A click with `held` on the composter at `pos`: `None` when it is no composter that takes it,
/// else whether the item is spent.
pub(super) fn fill(world: &mut World, layer: Entity, pos: BlockPos, held: &str) -> Option<bool> {
    let (id, filled) = filled(world, layer, pos)?;
    if filled == READY {
        return Some(!empty(world, layer, pos));
    }
    let chance = chance(held).filter(|_| filled < FULL)?;
    let position = Vec3f { x: pos[0] as f32, y: pos[1] as f32 + PARTICLES_UP, z: pos[2] as f32 };
    send::to_viewers(world, layer, pos, &[sapling_net::encode(&LevelEvent { event: LevelEventEvent::ParticleCropGrowth, position, data: 0 })]);
    // An empty one always takes the first: BDS's did in each run.
    let rises = filled == 0 || world.get_resource_or_insert_with(Dice::default).fraction() < chance;
    if rises {
        set_level(world, layer, pos, id, if filled + 1 == FULL { READY } else { filled + 1 });
    }
    hear(world, layer, pos, if rises { "block.composter.fill_success" } else { "block.composter.fill" });
    Some(true)
}

/// A click on a composter that is ready: it gives its bone meal. False for anything else.
pub(super) fn empty(world: &mut World, layer: Entity, pos: BlockPos) -> bool {
    let Some((id, READY)) = filled(world, layer, pos) else { return false };
    let meal = level(world, layer).and_then(|level| inventory::stored(&Item::new("minecraft:bone_meal".to_owned(), 1), level.registry()));
    set_level(world, layer, pos, id, 0);
    if let Some(meal) = meal {
        drops::spill(world, layer, meal, [pos[0], pos[1] + 1, pos[2]]);
    }
    hear(world, layer, pos, "block.composter.empty");
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_rots_has_a_chance_and_what_does_not_has_none() {
        assert_eq!([chance("minecraft:wheat_seeds"), chance("minecraft:wheat"), chance("minecraft:pumpkin_pie"), chance("minecraft:stone")], [Some(0.3), Some(0.65), Some(1.0), None]);
    }
}
