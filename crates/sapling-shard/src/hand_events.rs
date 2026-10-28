//! A player's hands and screens offered to plugins (sapling_api's hand_event.rs), each asked with
//! one line from its mechanic's module, which undoes a cancel on the client. An event nobody
//! hears is not built (life_events.rs `ask`).

use acacia_proto::packets::{LevelEvent, LevelEventEvent, ServerboundDiagnostics};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{
    Block, BlockPickEvent, BlockPos, ContainerSlotEvent, Diagnostics, DiagnosticsEvent, FireExtinguishEvent, ItemDamageEvent, ItemReleaseEvent, LecternPageTurnEvent, SetOnFireEvent,
    SlotChange, StartBreakEvent,
};
use sapling_world::ItemStack;

use crate::components::{Outgoing, PacketQueues, in_world_identity};
use crate::containers;
use crate::inventory::{Held, described};
use crate::life_events::{allowed, ask};

/// Whether plugins let `player` start breaking `block` at `pos`; a refused one has the crack
/// stopped on its screen.
pub(crate) fn may_start_break(world: &mut World, player: Entity, pos: BlockPos, block: Block) -> bool {
    if allowed(world, player, |_, who| StartBreakEvent::new(who, pos, block)) {
        return true;
    }
    let [x, y, z] = pos.map(|c| c as f32);
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.push(&LevelEvent { event: LevelEventEvent::BlockStopBreak, position: Vec3f { x, y, z }, data: 0 });
    }
    false
}

pub(crate) fn may_pick(world: &mut World, player: Entity, pos: BlockPos, block: Block) -> bool {
    allowed(world, player, |_, who| BlockPickEvent::new(who, pos, block))
}

pub(crate) fn may_release(world: &mut World, player: Entity, stack: &ItemStack, ticks: u64) -> bool {
    allowed(world, player, |_, who| ItemReleaseEvent::new(who, described(stack), ticks))
}

/// The uses a wear of `uses` on `held` counts once plugins had their say; 0: none.
pub(crate) fn wear(world: &mut World, player: Entity, held: &ItemStack, uses: i32) -> i32 {
    match ask(world, player, |_, who| ItemDamageEvent::new(who, described(held), uses)) {
        Some(event) if event.is_cancelled() => 0,
        Some(event) => event.damage.max(0),
        None => uses,
    }
}

/// The page plugins let `player` turn the lectern at `pos` to from `from`; `None`: it stays.
pub(crate) fn page_turned(world: &mut World, player: Entity, pos: BlockPos, from: i32, to: i32) -> Option<i32> {
    let [from, to] = [from, to].map(|page| u32::try_from(page).unwrap_or(0));
    match ask(world, player, |_, who| LecternPageTurnEvent::new(who, pos, from, to)) {
        Some(event) if event.is_cancelled() => None,
        Some(event) => i32::try_from(event.to).ok(),
        None => i32::try_from(to).ok(),
    }
}

pub(crate) fn may_burn(world: &mut World, player: Entity, ticks: u16) -> bool {
    allowed(world, player, |_, who| SetOnFireEvent::new(who, ticks))
}

pub(crate) fn may_extinguish(world: &mut World, player: Entity, pos: BlockPos) -> bool {
    allowed(world, player, |_, who| FireExtinguishEvent::new(who, pos))
}

/// What a request does to one slot of an open container: what left it, what came.
fn slot_changes(before: Option<&Held>, after: Option<&Held>) -> Vec<(SlotChange, ItemStack)> {
    let same = |a: &ItemStack, b: &ItemStack| a.name == b.name && a.damage == b.damage && a.tag == b.tag;
    let part = |held: &Held, count: u8| ItemStack { count, ..held.stack.clone() };
    match (before, after) {
        (Some(b), Some(a)) if same(&b.stack, &a.stack) && b.stack.count > a.stack.count => vec![(SlotChange::Take, part(b, b.stack.count - a.stack.count))],
        (Some(b), Some(a)) if same(&b.stack, &a.stack) && b.stack.count < a.stack.count => vec![(SlotChange::Place, part(a, a.stack.count - b.stack.count))],
        (Some(b), Some(a)) if same(&b.stack, &a.stack) => Vec::new(),
        _ => before.map(|b| (SlotChange::Take, b.stack.clone())).into_iter().chain(after.map(|a| (SlotChange::Place, a.stack.clone()))).collect(),
    }
}

/// Whether plugins let a request turn the open container's slots `before` into `after`; a take
/// of a stack the request throws out (`dropped`) is a drop.
pub(crate) fn may_change_container(world: &mut World, player: Entity, before: Option<&[Option<Held>]>, after: &[Option<Held>], dropped: &[ItemStack]) -> bool {
    let Some(before) = before.filter(|_| crate::plugins::listening::<ContainerSlotEvent>(world)) else { return true };
    let pos = containers::open_pos(world, player);
    let slots = before.len().max(after.len());
    let changes: Vec<(usize, SlotChange, ItemStack)> = (0..slots)
        .flat_map(|slot| {
            let changed = slot_changes(before.get(slot).and_then(Option::as_ref), after.get(slot).and_then(Option::as_ref));
            changed.into_iter().map(move |(change, stack)| (slot, if change == SlotChange::Take && dropped.iter().any(|thrown| thrown.name == stack.name && thrown.count == stack.count) { SlotChange::Drop } else { change }, stack))
        })
        .collect();
    changes.into_iter().all(|(slot, change, stack)| allowed(world, player, |_, who| ContainerSlotEvent::new(who, pos, slot, change, described(&stack))))
}

/// Offers this tick's client diagnostics.
pub(crate) fn dispatch_diagnostics(world: &mut World) {
    for (player, sent) in world.resource_mut::<PacketQueues>().take::<ServerboundDiagnostics>() {
        if in_world_identity(world, player).is_none() {
            continue;
        }
        let diagnostics = Diagnostics {
            frames_per_second: sent.average_frames_per_second,
            server_sim_tick_time: sent.average_server_sim_tick_time,
            client_sim_tick_time: sent.average_client_sim_tick_time,
            begin_frame_time: sent.average_begin_frame_time,
            input_time: sent.average_input_time,
            render_time: sent.average_render_time,
            end_frame_time: sent.average_end_frame_time,
            remainder_time_percent: sent.average_remainder_time_percent,
            unaccounted_time_percent: sent.average_unaccounted_time_percent,
        };
        ask(world, player, |_, who| DiagnosticsEvent::new(who, diagnostics));
    }
}
