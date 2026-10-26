//! What a player is shown of its open container a tick on, as BDS showed it again: a chest just
//! opened whole, and the slots a request changed (research/captures/bds-chest-moves.txt).

use bevy_ecs::prelude::*;

use super::screens::Viewing;
use super::slots::slot_shown;
use super::{content, slots_of};
use crate::components::{Outgoing, Tick};

/// When, to whom, under which window, and which slots (none: all of them).
#[derive(Resource, Default)]
pub(crate) struct ShownAgain(Vec<(u64, Entity, u8, Option<Vec<usize>>)>);

/// Shows `player` the `slots` (all, for none) of the container it has open under `window` again
/// on the next tick.
pub(super) fn next_tick(world: &mut World, player: Entity, window: u8, slots: Option<Vec<usize>>) {
    let due = world.resource::<Tick>().0 + 1;
    world.get_resource_or_insert_with(ShownAgain::default).0.push((due, player, window, slots));
}

/// Shows again what is due, to the players who still have that window open.
pub(crate) fn show_due(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let Some(mut again) = world.get_resource_mut::<ShownAgain>() else { return };
    let (due, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut again.0).into_iter().partition(|(when, ..)| *when <= now);
    again.0 = waiting;
    for (_, player, window, slots) in due {
        let still_open = world.get::<Viewing>(player).is_some_and(|viewing| viewing.window == window);
        let Some(held) = slots_of(world, player).filter(|_| still_open) else { continue };
        let Some(mut out) = world.get_mut::<Outgoing>(player) else { continue };
        match slots {
            None => out.push(&content(window, &held)),
            Some(slots) => slots.into_iter().filter(|slot| *slot < held.len()).for_each(|slot| out.push(&slot_shown(window, slot, &held[slot]))),
        }
    }
}
