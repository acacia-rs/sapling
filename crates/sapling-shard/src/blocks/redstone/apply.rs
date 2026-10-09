//! What blocks do with a new signal, by BDS's `_onRedstoneUpdate`s (docs/research/redstone.md 5,
//! 6): dust stores it, a lamp lights at once and goes out 4 ticks after it ends, doors, trapdoors
//! and gates open with their sound, a hopper locks, a torch flips.

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::parts::{Opens, Part, SIGNAL, User};
use super::{Circuits, Due, power_at};
use crate::blocks::{doors, level, send, with_property};
use crate::components::Tick;

/// Ticks a lamp stays lit once its signal ended (`RedstoneLampBlock`, a queued tick of 4).
const LAMP_DELAY: u64 = 4;
const LIT_LAMP: &str = "minecraft:lit_redstone_lamp";
const LAMP: &str = "minecraft:redstone_lamp";

/// The block at `pos`: its id and what it is to a circuit.
fn at(world: &World, layer: Entity, [x, y, z]: BlockPos) -> Option<(u32, Part)> {
    let level = level(world, layer).filter(|level| level.in_height(y) && level.is_loaded(send::column([x, y, z])))?;
    let id = level.block(x, y, z);
    Some((id, Part::of(level.registry(), id)))
}

/// The state of the block at `pos` with `key` set to `value`.
fn with(world: &World, layer: Entity, [x, y, z]: BlockPos, key: &str, value: &str) -> Option<u32> {
    let level = level(world, layer)?;
    with_property(level.registry(), level.block(x, y, z), key, value)
}

/// The same block's state under another name: a lamp or torch lit or not.
fn renamed(world: &World, layer: Entity, id: u32, name: &str) -> Option<u32> {
    let registry = level(world, layer)?.registry();
    registry.find(name, registry.get(id)?.properties)
}

fn play(world: &mut World, layer: Entity, pos: BlockPos, name: &str) {
    let Some(level) = level(world, layer) else { return };
    let heard = send::sound(name, level.registry(), pos, level.block(pos[0], pos[1], pos[2]));
    send::to_viewers(world, layer, pos, &[heard]);
}

pub(super) fn write(world: &mut World, layer: Entity, pos: BlockPos, due: Due) {
    let Some((id, part)) = at(world, layer, pos) else { return };
    match (part, due) {
        (Part::Dust { signal }, Due::Signal(new)) if signal != new => {
            if let Some(id) = with(world, layer, pos, SIGNAL, &new.to_string()) {
                send::set(world, layer, pos, id);
            }
        }
        (Part::User(user), Due::Signal(signal)) => use_signal(world, layer, pos, id, user, signal),
        (Part::Torch { lit, .. } | Part::Repeater { lit, .. }, Due::Lit { lit: now, fizz }) if lit != now => {
            let is_torch = matches!(part, Part::Torch { .. });
            let names = if is_torch { ["minecraft:unlit_redstone_torch", "minecraft:redstone_torch"] } else { ["minecraft:unpowered_repeater", "minecraft:powered_repeater"] };
            let name = names[usize::from(now)];
            if fizz {
                play(world, layer, pos, "fizz");
            }
            if let Some(id) = renamed(world, layer, id, name) {
                send::set(world, layer, pos, id);
            }
        }
        _ => {}
    }
}

fn use_signal(world: &mut World, layer: Entity, pos: BlockPos, id: u32, user: User, signal: u8) {
    match user {
        User::Lamp { lit } if signal > 0 => {
            world.resource_mut::<Circuits>().lamps_off.retain(|(_, at_layer, at)| (*at_layer, *at) != (layer, pos));
            if let Some(lit_id) = renamed(world, layer, id, LIT_LAMP).filter(|_| !lit) {
                send::set(world, layer, pos, lit_id);
            }
        }
        User::Lamp { lit: true } => {
            let when = world.resource::<Tick>().0 + LAMP_DELAY;
            let mut circuits = world.resource_mut::<Circuits>();
            if !circuits.lamps_off.iter().any(|(_, at_layer, at)| (*at_layer, *at) == (layer, pos)) {
                circuits.lamps_off.push((when, layer, pos));
            }
        }
        User::Lamp { lit: false } => {}
        User::Hopper => {
            if let Some(id) = with(world, layer, pos, "toggle_bit", if signal > 0 { "1" } else { "0" }) {
                send::set(world, layer, pos, id);
            }
        }
        User::Opens(opens) => open(world, layer, pos, opens, signal),
    }
}

/// Opens or shuts a door, trapdoor or gate to match its signal, with its sound. A door is powered
/// by either half and only its lower half stores `open_bit`; its upper is shown again with it.
fn open(world: &mut World, layer: Entity, pos: BlockPos, opens: Opens, signal: u8) {
    let (lower, upper) = match doors::other_half(world, layer, pos).filter(|_| opens == Opens::Door) {
        Some(other) if other[1] < pos[1] => (other, Some(pos)),
        other => (pos, other),
    };
    let powered = signal > 0 || [Some(lower), upper].into_iter().flatten().any(|half| power_at(world, layer, half) > 0);
    let Some(id) = with(world, layer, lower, "open_bit", if powered { "1" } else { "0" }) else { return };
    if !send::set(world, layer, lower, id) {
        return;
    }
    if let Some(upper) = upper {
        send::show(world, layer, upper);
    }
    let sound = match opens {
        Opens::Door => ["door.open", "door.close"],
        Opens::Trapdoor => ["trapdoor.open", "trapdoor.close"],
        Opens::Gate => ["fence_gate.open", "fence_gate.close"],
    };
    play(world, layer, lower, sound[usize::from(!powered)]);
}

/// Puts out the lamps whose delay is up and that still get no signal.
pub(super) fn put_out_lamps(world: &mut World) {
    let tick = world.resource::<Tick>().0;
    let mut circuits = world.resource_mut::<Circuits>();
    let (due, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut circuits.lamps_off).into_iter().partition(|(when, ..)| *when <= tick);
    circuits.lamps_off = waiting;
    for (_, layer, pos) in due {
        let Some((id, Part::User(User::Lamp { lit: true }))) = at(world, layer, pos) else { continue };
        if power_at(world, layer, pos) == 0
            && let Some(id) = renamed(world, layer, id, LAMP)
        {
            send::set(world, layer, pos, id);
        }
    }
}
