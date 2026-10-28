//! Coral dying out of water (docs/research/world-ticks.md, "Coral"): live coral, its blocks, fans
//! and wall fans with no water in or beside them wait 60 to 99 ticks on BDS's queued tick (the
//! `BlockQueuedTickEvent` CoralBlock and CoralPlantBlock subscribe to) and then turn dead, unless
//! water came back meanwhile.

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};

use super::queued::{self, Job};
use super::{Dice, level, send};

const TYPES: [&str; 5] = ["tube", "brain", "bubble", "fire", "horn"];
const FORMS: [&str; 4] = ["coral", "coral_block", "coral_fan", "coral_wall_fan"];
/// The wait before a coral out of water dies: from this many ticks, to this many more (Java's
/// `CoralBlock`, `BaseCoralPlantTypeBlock`).
const WAIT: u64 = 60;
const WAIT_SPREAD: u32 = 40;

type Level = sapling_world::World;

/// Whether `name` is live coral, block, plant or fan.
pub(super) fn is_live(name: &str) -> bool {
    let Some(bare) = name.strip_prefix("minecraft:") else { return false };
    TYPES.iter().any(|kind| bare.strip_prefix(kind).and_then(|rest| rest.strip_prefix('_')).is_some_and(|form| FORMS.contains(&form)))
}

fn live_at(level: &Level, [x, y, z]: BlockPos) -> Option<&'static str> {
    level.in_height(y).then(|| level.registry().get(level.block(x, y, z))).flatten().map(|state| state.name).filter(|name| is_live(name))
}

fn holds_water(level: &Level, [x, y, z]: BlockPos) -> bool {
    if !level.in_height(y) || !level.is_loaded(send::column([x, y, z])) {
        return true;
    }
    let is_water = |id: u32| level.registry().get(id).is_some_and(|state| matches!(state.name, "minecraft:water" | "minecraft:flowing_water" | "minecraft:bubble_column"));
    is_water(level.block(x, y, z)) || is_water(level.liquid(x, y, z))
}

/// Whether the coral at `pos` has water in its cell or one beside it; a coral block's own cell
/// holds none.
fn is_wet(level: &Level, pos: BlockPos) -> bool {
    holds_water(level, pos) || Face::ALL.into_iter().any(|face| holds_water(level, face.neighbour(pos)))
}

/// After a write at `pos`: it and the coral beside it that are dry now wait to die.
pub(super) fn written(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(level) = level(world, layer) else { return };
    let dry: Vec<BlockPos> = std::iter::once(pos).chain(Face::ALL.map(|face| face.neighbour(pos))).filter(|cell| live_at(level, *cell).is_some() && !is_wet(level, *cell)).collect();
    for cell in dry {
        let delay = WAIT + u64::from(world.get_resource_or_insert_with(Dice::default).roll(WAIT_SPREAD));
        queued::again(world, layer, cell, delay, Job::Coral);
    }
}

/// The coral at `pos`, if still live and dry, turns dead, keeping its facing.
pub(super) fn die(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(level) = level(world, layer) else { return };
    let Some(name) = live_at(level, pos).filter(|_| !is_wet(level, pos)) else { return };
    let [x, y, z] = pos;
    let dead = format!("minecraft:dead_{}", &name["minecraft:".len()..]);
    let Some(id) = level.registry().get(level.block(x, y, z)).and_then(|state| level.registry().find(&dead, state.properties)) else { return };
    send::set(world, layer, pos, id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_live_coral_has_a_dead_twin() {
        let registry = sapling_world::BlockRegistry::vanilla();
        for kind in TYPES {
            for form in FORMS {
                let name = format!("minecraft:{kind}_{form}");
                assert!(is_live(&name) && registry.states_of(&name).next().is_some(), "{name}");
                assert!(registry.states_of(&format!("minecraft:dead_{kind}_{form}")).next().is_some(), "dead {name}");
            }
        }
        assert!(!is_live("minecraft:dead_tube_coral") && !is_live("minecraft:coral_block"));
    }
}
