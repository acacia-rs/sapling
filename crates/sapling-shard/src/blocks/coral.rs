//! Coral dying out of water (docs/research/world-ticks.md, "Coral"): live coral, its blocks, fans
//! and wall fans out of water wait on BDS's queued tick and then turn dead, unless water came back
//! meanwhile. Each write in or beside a coral queues its own tick; none replaces another
//! (`BlockTickingQueue::add` 1415f50f0 keeps every entry).

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};

use super::queued::{self, Job};
use super::{Dice, level, send};

const TYPES: [&str; 5] = ["tube", "brain", "bubble", "fire", "horn"];
const FORMS: [&str; 4] = ["coral", "coral_block", "coral_fan", "coral_wall_fan"];
/// A dry coral block's wait once written (`CoralBlock` place 14713b7c0; nine of nine at 45 in
/// research/captures/bds-t3odds.txt).
const BLOCK_WAIT: u64 = 45;
/// A plant's wait once something beside it changed (`CoralPlantBlock::neighborChanged` 14713ebb0).
const PLANT_NUDGED: u64 = 5;
/// The long wait, from this many ticks to this many more: a plant written dry or still wet at its
/// tick, a block or fan beside a change (14713e660, 14713bd50, 14713c770).
const WAIT: u64 = 40;
const WAIT_SPREAD: u32 = 160;

type Level = sapling_world::World;

#[derive(Clone, Copy, PartialEq)]
enum Form {
    Plant,
    Block,
    Fan,
}

/// Whether `name` is live coral, block, plant or fan.
pub(super) fn is_live(name: &str) -> bool {
    form(name).is_some()
}

fn form(name: &str) -> Option<Form> {
    let bare = name.strip_prefix("minecraft:")?;
    let form = TYPES.iter().find_map(|kind| bare.strip_prefix(kind).and_then(|rest| rest.strip_prefix('_')).filter(|form| FORMS.contains(form)))?;
    Some(match form {
        "coral" => Form::Plant,
        "coral_block" => Form::Block,
        _ => Form::Fan,
    })
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

/// What a coral fan put by hand at `pos` becomes, the placer's feet in cell `feet`: dead where its cell
/// holds no water, and `coral_fan_direction` 0 only where `atan2(dz, dx)` from the feet to it lies
/// between 45 and 135 degrees; PocketMine's `CoralFan::place`, which all nine of BDS's hand-placed fans
/// fit (research/captures/bds-g3fan.txt, bds-e3coral.txt). Anything else is `id` as it is.
pub(super) fn placed(level: &Level, pos: BlockPos, id: u32, feet: BlockPos) -> u32 {
    let registry = level.registry();
    let Some(state) = registry.get(id).filter(|state| state.name.ends_with("_coral_fan")) else { return id };
    let name = match is_live(state.name) && !holds_water(level, pos) {
        true => format!("minecraft:dead_{}", &state.name["minecraft:".len()..]),
        false => state.name.to_owned(),
    };
    let angle = ((pos[2] - feet[2]) as f32).atan2((pos[0] - feet[0]) as f32).to_degrees();
    let direction = if angle > 45.0 && angle < 135.0 { 0 } else { 1 };
    registry.find(&name, &format!("coral_fan_direction={direction}")).unwrap_or(id)
}

fn long_wait(world: &mut World) -> u64 {
    WAIT + u64::from(world.get_resource_or_insert_with(Dice::default).roll(WAIT_SPREAD))
}

/// After a write at `pos`: a coral written there and the coral beside it queue their ticks.
// TODO: a fan's own place tick was not decompiled; it keeps the long wait when written dry.
pub(super) fn written(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(level) = level(world, layer) else { return };
    let here = live_at(level, pos).and_then(form).filter(|form| match form {
        Form::Plant => !holds_water(level, pos),
        Form::Block | Form::Fan => !is_wet(level, pos),
    });
    let beside: Vec<(BlockPos, Form)> = Face::ALL.into_iter().map(|face| face.neighbour(pos)).filter_map(|cell| Some((cell, form(live_at(level, cell)?)?))).collect();
    if let Some(form) = here {
        let delay = if form == Form::Block { BLOCK_WAIT } else { long_wait(world) };
        queued::after(world, layer, pos, delay, Job::Coral);
    }
    for (cell, form) in beside {
        let delay = if form == Form::Plant { PLANT_NUDGED } else { long_wait(world) };
        queued::after(world, layer, cell, delay, Job::Coral);
    }
}

/// The coral at `pos`, if still live and dry, turns dead, keeping its facing; a plant still wet
/// waits again (14713e950).
pub(super) fn die(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(level) = level(world, layer) else { return };
    let Some(name) = live_at(level, pos) else { return };
    if is_wet(level, pos) {
        if form(name) == Some(Form::Plant) {
            let delay = long_wait(world);
            queued::after(world, layer, pos, delay, Job::Coral);
        }
        return;
    }
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
