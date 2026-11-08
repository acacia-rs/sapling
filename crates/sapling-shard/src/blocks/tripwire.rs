//! Tripwire hooks and their wire (docs/research/redstone-blocks.md, "Tripwire"): two hooks facing
//! each other across a line of wire are attached; anything in a wire presses it, and the hooks then
//! power the blocks they hang on. A pressed wire looks again every 10 ticks until it is left.

use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};

use super::plates::bodies;
use super::queued::{self, Job};
use super::{level, send, with_property};

const HOOK: &str = "minecraft:tripwire_hook";
const WIRE: &str = "minecraft:trip_wire";
/// Cells from a hook to the farthest hook it can join, the wire between them.
const REACH: i32 = 42;
const RECHECK: u64 = 10;
/// How high in its cell a wire is felt.
const FELT: f32 = 0.15625;
const ACROSS: [Face; 4] = [Face::North, Face::South, Face::West, Face::East];

/// The name and a property of the block at `pos`.
fn state(world: &World, layer: Entity, [x, y, z]: BlockPos) -> Option<(&'static str, &'static str, u32)> {
    let level = level(world, layer).filter(|level| level.in_height(y) && level.is_loaded(send::column([x, y, z])))?;
    let id = level.block(x, y, z);
    let state = level.registry().get(id)?;
    Some((state.name, state.properties, id))
}

fn bit(properties: &str, key: &str) -> bool {
    properties.split(',').any(|pair| pair.strip_prefix(key).and_then(|rest| rest.strip_prefix('=')) == Some("1"))
}

/// The way a hook's `direction` faces: along its wire.
fn facing(properties: &str) -> Option<Face> {
    let direction = properties.split(',').find_map(|pair| pair.strip_prefix("direction="))?;
    Some(match direction {
        "0" => Face::South,
        "1" => Face::West,
        "2" => Face::North,
        "3" => Face::East,
        _ => return None,
    })
}

fn set_bits(world: &mut World, layer: Entity, pos: BlockPos, bits: &[(&str, bool)]) -> bool {
    let Some((_, _, mut id)) = state(world, layer, pos) else { return false };
    let Some(registry) = level(world, layer).map(|level| level.registry()) else { return false };
    for (key, on) in bits {
        id = with_property(registry, id, key, if *on { "1" } else { "0" }).unwrap_or(id);
    }
    send::set(world, layer, pos, id)
}

/// Whether a body lies in the wire at `pos`.
fn occupied(bodies: &[super::plates::Body], layer: Entity, [x, y, z]: BlockPos) -> bool {
    let (min, max) = ([x as f32, y as f32, z as f32], [x as f32 + 1.0, y as f32 + FELT, z as f32 + 1.0]);
    bodies.iter().any(|body| body.layer == layer && !body.spectator && (0..3).all(|axis| body.min[axis] < max[axis] && body.max[axis] > min[axis]))
}

/// Presses the wires something stands in.
pub(crate) fn tick(world: &mut World) {
    let bodies = bodies(world);
    let mut touched: Vec<(Entity, BlockPos)> = bodies.iter().filter(|body| !body.spectator).flat_map(|body| body.cells().map(|cell| (body.layer, cell))).collect();
    touched.sort_unstable();
    touched.dedup();
    for (layer, pos) in touched {
        let Some((WIRE, properties, _)) = state(world, layer, pos) else { continue };
        if !bit(properties, "powered_bit") && occupied(&bodies, layer, pos) {
            set_bits(world, layer, pos, &[("powered_bit", true)]);
            queued::after(world, layer, pos, RECHECK, Job::Tripwire);
            wire_changed(world, layer, pos);
        }
    }
}

/// A pressed wire's check: still pressed, it looks again later; left, it lets its hooks go. Where
/// the wire is gone, its hooks look at their lines (B22: the tick outlives the wire).
pub(super) fn recheck(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some((WIRE, properties, _)) = state(world, layer, pos) else {
        wire_changed(world, layer, pos);
        return;
    };
    if !bit(properties, "powered_bit") {
        return;
    }
    if occupied(&bodies(world), layer, pos) {
        queued::after(world, layer, pos, RECHECK, Job::Tripwire);
        return;
    }
    set_bits(world, layer, pos, &[("powered_bit", false)]);
    wire_changed(world, layer, pos);
}

/// The hooks whose lines run through the wire at `pos` look at their line again.
fn wire_changed(world: &mut World, layer: Entity, pos: BlockPos) {
    for face in ACROSS {
        for step in 1..REACH {
            let at = [0, 1, 2].map(|axis| pos[axis] + face.offset()[axis] * step);
            match state(world, layer, at) {
                Some((WIRE, ..)) => continue,
                Some((HOOK, properties, _)) if facing(properties) == Some(super::opposite(face)) => update_line(world, layer, at),
                _ => {}
            }
            break;
        }
    }
}

/// Joins the wire at `pos` toward each wire beside it and each hook facing it, and suspends it, as
/// BDS wrote every wire put by hand or set (bds-z2wire.txt).
fn shape(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some((WIRE, ..)) = state(world, layer, pos) else { return };
    let joins = ACROSS.map(|face| match state(world, layer, face.neighbour(pos)) {
        Some((WIRE, ..)) => true,
        Some((HOOK, properties, _)) => facing(properties) == Some(super::opposite(face)),
        _ => false,
    });
    let bits = [("minecraft:connection_north", joins[0]), ("minecraft:connection_south", joins[1]), ("minecraft:connection_west", joins[2]), ("minecraft:connection_east", joins[3]), ("suspended_bit", true)];
    set_bits(world, layer, pos, &bits);
}

/// A hook, a wire or what was one was written at `pos`: it and the wires beside it take their
/// shapes; a new hook looks at its line, a new wire has the hooks of the lines through it look at
/// theirs. A wire or hook taken away tells no hook (bds-i1trip.txt; B16's `onRemove` changed nothing).
pub(super) fn written(world: &mut World, layer: Entity, pos: BlockPos, old: &str, new: &str) {
    if old == new || ![old, new].iter().any(|name| *name == HOOK || *name == WIRE) {
        return;
    }
    shape(world, layer, pos);
    ACROSS.iter().for_each(|face| shape(world, layer, face.neighbour(pos)));
    match new {
        HOOK => update_line(world, layer, pos),
        WIRE => wire_changed(world, layer, pos),
        _ => {}
    }
}

/// A player is breaking the wire at `pos` (B24): without shears it is pressed first, and its hooks
/// look at their lines now and again 10 ticks on, when the wire is gone (bds-i1trip.txt).
pub(super) fn breaking(world: &mut World, layer: Entity, pos: BlockPos, shears: bool) {
    let Some((WIRE, ..)) = state(world, layer, pos) else { return };
    set_bits(world, layer, pos, &[("powered_bit", !shears)]);
    wire_changed(world, layer, pos);
    queued::after(world, layer, pos, RECHECK, Job::Tripwire);
}

/// Attaches the hook at `hook` to the hook facing it across a line of wire, or lets it go, and
/// powers both while a wire of the line is pressed and not disarmed.
fn update_line(world: &mut World, layer: Entity, hook: BlockPos) {
    let Some((HOOK, properties, _)) = state(world, layer, hook) else { return };
    let Some(face) = facing(properties) else { return };
    let (was_attached, was_powered) = (bit(properties, "attached_bit"), bit(properties, "powered_bit"));
    let mut wires = Vec::new();
    let mut other = None;
    for step in 1..REACH {
        let at = [0, 1, 2].map(|axis| hook[axis] + face.offset()[axis] * step);
        match state(world, layer, at) {
            Some((WIRE, properties, _)) => wires.push((at, bit(properties, "powered_bit") && !bit(properties, "disarmed_bit"))),
            Some((HOOK, properties, _)) if facing(properties) == Some(super::opposite(face)) && step > 1 => {
                other = Some(at);
                break;
            }
            _ => break,
        }
    }
    let attached = other.is_some();
    let powered = attached && wires.iter().any(|(_, pressed)| *pressed);
    // The far hook first, each heard at its own cell, as BDS did (B18; bds-z2wire.txt).
    let ends: Vec<BlockPos> = [other, Some(hook)].into_iter().flatten().collect();
    for end in &ends {
        set_bits(world, layer, *end, &[("attached_bit", attached), ("powered_bit", powered)]);
    }
    // B18: the wires are rewritten only as the line attaches or lets go.
    for (wire, _) in wires.iter().filter(|_| attached != was_attached) {
        set_bits(world, layer, *wire, &[("attached_bit", attached)]);
    }
    let sound = match (was_attached, attached, was_powered, powered) {
        (_, _, false, true) => Some("power.on"),
        (_, _, true, false) => Some("power.off"),
        (false, true, ..) => Some("attach"),
        (true, false, ..) => Some("detach"),
        _ => None,
    };
    let Some((name, registry)) = sound.zip(level(world, layer).map(|level| level.registry())) else { return };
    let heard: Vec<bytes::Bytes> = ends.iter().map(|[x, y, z]| send::sound_at(name, registry, Vec3f { x: *x as f32 + 0.5, y: *y as f32 + 0.1, z: *z as f32 + 0.5 }, registry.air_id())).collect();
    send::to_viewers(world, layer, hook, &heard);
}
