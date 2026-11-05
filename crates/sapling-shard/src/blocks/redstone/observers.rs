//! Observers: one watching a cell that changed is powered 2 ticks later, for 2 ticks
//! (docs/research/redstone.md, "Observers").

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};

use super::Circuits;
use super::parts::{Part, opposite};
use crate::blocks::{level, send, with_property};
use crate::components::Tick;

/// Ticks from a change to the observer's pulse, and its length.
const DELAY: u64 = 2;
const PULSE: u64 = 2;

#[derive(Default)]
pub(super) struct Watch {
    /// Cells written and the tick they were written in, not yet looked at.
    pub(super) written: Vec<(u64, Entity, BlockPos)>,
    /// When each observer about to pulse, or pulsing, turns on or off.
    pulses: Vec<(u64, Entity, BlockPos, bool)>,
}

fn observer_at(world: &World, layer: Entity, [x, y, z]: BlockPos) -> Option<(u32, Face)> {
    let level = level(world, layer).filter(|level| level.in_height(y) && level.is_loaded(send::column([x, y, z])))?;
    let id = level.block(x, y, z);
    match Part::of(level.registry(), id) {
        Part::Observer { watch, .. } => Some((id, watch)),
        _ => None,
    }
}

/// Starts those that watch a cell written since the last tick, and turns on and off the observers whose
/// time has come, in the order their turns were set, each turn starting the observers watching it at once
/// and an observer turned on setting its turn off after that. One with a turn to come takes no notice: in
/// Java's order, two observers watching each other pulse every 6 ticks, as BDS's did (bds-p2rs.txt).
pub(super) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    start_watching(world);
    loop {
        let Some(mut circuits) = world.get_resource_mut::<Circuits>() else { return };
        let pulses = &mut circuits.observers.pulses;
        let Some(next) = pulses.iter().position(|(when, ..)| *when <= now) else { return };
        let (_, layer, pos, on) = pulses.remove(next);
        let Some((id, _)) = observer_at(world, layer, pos) else { continue };
        let registry = level(world, layer).map(|level| level.registry());
        if let Some(id) = registry.and_then(|registry| with_property(registry, id, "powered_bit", if on { "1" } else { "0" })) {
            send::set(world, layer, pos, id);
        }
        start_watching(world);
        if on {
            world.resource_mut::<Circuits>().observers.pulses.push((now + PULSE, layer, pos, false));
        }
    }
}

/// Starts each observer watching a cell written since this was last asked that has no turn to come.
fn start_watching(world: &mut World) {
    let written = std::mem::take(&mut world.resource_mut::<Circuits>().observers.written);
    for (at, layer, pos) in written {
        for face in Face::ALL {
            let observer = face.neighbour(pos);
            if observer_at(world, layer, observer).is_none_or(|(_, watch)| watch != opposite(face)) {
                continue;
            }
            let pulses = &mut world.resource_mut::<Circuits>().into_inner().observers.pulses;
            if !pulses.iter().any(|(_, at_layer, at_pos, _)| (*at_layer, *at_pos) == (layer, observer)) {
                pulses.push((at + DELAY, layer, observer, true));
            }
        }
    }
}
