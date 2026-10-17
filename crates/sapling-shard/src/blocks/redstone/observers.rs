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

/// Turns on and off the observers whose time has come, and starts those that watch a cell
/// written since the last tick; one already started takes no notice.
pub(super) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let Some(mut circuits) = world.get_resource_mut::<Circuits>() else { return };
    let watch = &mut circuits.observers;
    let (due, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut watch.pulses).into_iter().partition(|(when, ..)| *when <= now);
    watch.pulses = waiting;
    for (_, layer, pos, on) in due {
        let Some((id, _)) = observer_at(world, layer, pos) else { continue };
        let registry = level(world, layer).map(|level| level.registry());
        if let Some(id) = registry.and_then(|registry| with_property(registry, id, "powered_bit", if on { "1" } else { "0" })) {
            send::set(world, layer, pos, id);
        }
    }
    let written = std::mem::take(&mut world.resource_mut::<Circuits>().observers.written);
    for (at, layer, pos) in written {
        for face in Face::ALL {
            let observer = face.neighbour(pos);
            if observer_at(world, layer, observer).is_none_or(|(_, watch)| watch != opposite(face)) {
                continue;
            }
            let pulses = &mut world.resource_mut::<Circuits>().into_inner().observers.pulses;
            if !pulses.iter().any(|(_, at_layer, at_pos, _)| (*at_layer, *at_pos) == (layer, observer)) {
                pulses.extend([(at + DELAY, layer, observer, true), (at + DELAY + PULSE, layer, observer, false)]);
            }
        }
    }
}
