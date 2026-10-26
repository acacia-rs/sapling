//! Crafters (docs/research/dispensers.md, "Crafter"): fired as a dispenser is, one crafts the
//! crafting-table recipe its 3x3 grid lays out, uses one of each filled slot and puts the result
//! into the container it faces or throws it out. Slots a player disabled stay empty; a hopper
//! fills the enabled slot holding fewest. A comparator reads its filled and disabled slots.

use acacia_proto::nbt::Value;
use acacia_proto::packets::{LevelEventEvent, ToggleCrafterSlotRequest};
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};
use sapling_world::ItemStack;

use super::queued::{self, Job};
use super::{fields, level, send, with_property};
use crate::components::{InLayer, PacketQueues};
use crate::{containers, hoppers};

const CRAFTER: &str = "minecraft:crafter";
const ENTITY: &str = "Crafter";
const DISABLED: &str = "disabled_slots";
const SLOTS: usize = 9;
/// How long the `crafting` light stays on after a craft.
const LIT: u64 = 6;

/// The state of the block at `pos` if it is a crafter, with its id.
fn crafter(world: &World, layer: Entity, [x, y, z]: BlockPos) -> Option<(u32, &'static str)> {
    let level = level(world, layer).filter(|level| level.in_height(y))?;
    let id = level.block(x, y, z);
    level.registry().get(id).filter(|state| state.name == CRAFTER).map(|state| (id, state.properties))
}

/// The way the crafter at `pos` faces: the first half of its `orientation` (`north_up`, `down_east`).
pub(super) fn facing(world: &World, layer: Entity, pos: BlockPos) -> Option<Face> {
    let (_, properties) = crafter(world, layer, pos)?;
    let orientation = properties.split(',').find_map(|pair| pair.strip_prefix("orientation="))?;
    match orientation.trim_matches('"').split('_').next()? {
        "down" => Some(Face::Down),
        "up" => Some(Face::Up),
        "north" => Some(Face::North),
        "south" => Some(Face::South),
        "west" => Some(Face::West),
        "east" => Some(Face::East),
        _ => None,
    }
}

/// The slots of the crafter at `pos` a player disabled, a bit each.
fn disabled(world: &World, layer: Entity, pos: BlockPos) -> u16 {
    match fields::get(world, layer, pos, DISABLED) {
        Some(Value::Short(bits)) => bits as u16,
        _ => 0,
    }
}

/// What a comparator reads off the crafter at `pos`: one for each slot filled or disabled.
pub(super) fn signal(world: &mut World, layer: Entity, pos: BlockPos) -> Option<u8> {
    crafter(world, layer, pos)?;
    let off = disabled(world, layer, pos);
    let slots = containers::contents(world, layer, pos).unwrap_or_default();
    Some((0..SLOTS).filter(|slot| off & 1 << slot != 0 || slots.get(*slot).is_some_and(Option::is_some)).count() as u8)
}

/// The slot a hopper puts `item` into at `pos`: `None` for a block that is no crafter, `Some(None)`
/// for a crafter with no slot for it. The enabled slot holding fewest takes it, the first of equals.
pub(crate) fn crafter_slot(world: &mut World, layer: Entity, pos: BlockPos, item: &ItemStack) -> Option<Option<usize>> {
    crafter(world, layer, pos)?;
    let off = disabled(world, layer, pos);
    let slots = containers::contents(world, layer, pos).unwrap_or_default();
    let max = sapling_data::max_stack(&item.name);
    let room = |slot: usize| match slots.get(slot).and_then(Option::as_ref) {
        None => Some(0),
        Some(held) if held.stack.name == item.name && held.stack.damage == item.damage && held.stack.tag == item.tag && held.stack.count < max => Some(held.stack.count),
        Some(_) => None,
    };
    Some((0..SLOTS).filter(|slot| off & 1 << slot == 0).filter_map(|slot| Some((room(slot)?, slot))).min().map(|(_, slot)| slot))
}

fn play(world: &mut World, layer: Entity, pos: BlockPos, sound: &str) {
    let Some(level) = level(world, layer) else { return };
    let noise = send::sound(sound, level.registry(), pos, level.block(pos[0], pos[1], pos[2]));
    send::to_viewers(world, layer, pos, &[noise]);
}

/// The crafter at `pos`, facing `face`, fired: it crafts what its grid lays out, or fails.
pub(super) fn craft(world: &mut World, layer: Entity, pos: BlockPos, face: Face) {
    let slots = containers::contents(world, layer, pos).unwrap_or_default();
    let grid: Vec<Option<(&str, i16)>> = (0..SLOTS).map(|slot| slots.get(slot).and_then(Option::as_ref).map(|held| (held.stack.name.as_str(), held.stack.damage))).collect();
    let Some(recipe) = sapling_data::laid_out_in(&grid) else { return play(world, layer, pos, "crafter.fail") };
    let results: Vec<ItemStack> = recipe.outputs.iter().map(|(made, count)| ItemStack { name: made.name.to_owned(), count: *count, damage: made.damage, block: made.block, tag: made.tag.clone() }).collect();
    containers::with_slots(world, layer, pos, |_, slots| {
        for slot in slots.iter_mut().take(SLOTS) {
            let Some(held) = slot.as_mut() else { continue };
            held.stack.count -= 1;
            if held.stack.count == 0 {
                *slot = None;
            }
        }
    });
    super::container_changed(world, layer, pos);
    if let Some((id, _)) = crafter(world, layer, pos) {
        let lit = level(world, layer).and_then(|level| with_property(level.registry(), id, "crafting", "1"));
        if let Some(lit) = lit {
            send::set(world, layer, pos, lit);
        }
    }
    queued::after(world, layer, pos, LIT, Job::Crafted);
    let mut thrown = false;
    for result in results {
        thrown |= eject(world, layer, pos, face, result);
    }
    if thrown {
        play(world, layer, pos, "crafter.craft");
    }
}

/// Puts `result` into the container in front of the crafter, one by one, and throws out the rest
/// with white smoke; true if any was thrown (D25).
fn eject(world: &mut World, layer: Entity, pos: BlockPos, face: Face, mut result: ItemStack) -> bool {
    let front = face.neighbour(pos);
    if hoppers::takes_items(world, layer, front) {
        while result.count > 0 && hoppers::put_one(world, layer, front, super::opposite(face), &result) {
            result.count -= 1;
        }
    }
    if result.count == 0 {
        return false;
    }
    super::dispensers::spit(world, layer, pos, face, result);
    super::dispensers::smoke(world, layer, pos, face, LevelEventEvent::ParticleShootWhiteSmoke);
    true
}

/// The crafter's `crafting` light goes out.
pub(super) fn crafted(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some((id, _)) = crafter(world, layer, pos) else { return };
    let Some(out) = level(world, layer).and_then(|level| with_property(level.registry(), id, "crafting", "0")) else { return };
    send::set(world, layer, pos, out);
}

/// Players turning slots of the crafter they have open on and off; an empty slot alone turns off.
pub(crate) fn dispatch_toggles(world: &mut World) {
    for (player, request) in world.resource_mut::<PacketQueues>().take::<ToggleCrafterSlotRequest>() {
        let Some(layer) = world.get::<InLayer>(player).map(|layer| layer.0) else { continue };
        let pos = [request.position.x, request.position.y, request.position.z];
        let slot = usize::from(request.slot);
        let is_open = containers::viewers(world, layer, pos).iter().any(|(viewer, _)| *viewer == player);
        let is_empty = containers::contents(world, layer, pos).is_some_and(|slots| slots.get(slot).is_some_and(Option::is_none));
        if slot >= SLOTS || !is_open || crafter(world, layer, pos).is_none() || request.disabled && !is_empty {
            continue;
        }
        let bits = disabled(world, layer, pos);
        let bits = if request.disabled { bits | 1 << slot } else { bits & !(1 << slot) };
        fields::set(world, layer, pos, ENTITY, DISABLED, Some(Value::Short(bits as i16)));
        if request.disabled {
            play(world, layer, pos, "crafter.disable_slot");
        }
        super::container_changed(world, layer, pos);
    }
}
