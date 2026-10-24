//! Pistons, by what BDS shows of them (docs/research/redstone.md, "Pistons"): a powered one
//! pushes up to 12 blocks over four ticks, breaking what has no body, and an unpowered one draws
//! back, a sticky one bringing the block before it. Its block entity tells clients where it is.
// TODO: a piston that could not push does not try again
// until its power changes; a container open on a screen is moved unseen;
// what breaks bursts a tick later than on BDS, and what stood on a pushed block falls a tick later.

use acacia_proto::nbt::{List, Str, Value};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};
use sapling_world::{block_of_hash, new_block_entity};

use super::{level, send, updates};

mod plan;
mod shove;
use crate::components::Tick;

/// The side a piston with this `facing_direction` pushes to.
pub(super) fn pushes(facing: &str) -> Option<Face> {
    PUSHES.get(facing.parse::<usize>().ok()?).copied()
}

const MOVING: &str = "minecraft:moving_block";
/// A piston's `facing_direction` and the side it pushes to (research/captures/bds-redstone-piston-facing.txt).
const PUSHES: [Face; 6] = [Face::Down, Face::Up, Face::South, Face::North, Face::East, Face::West];
/// The `State` of a piston's block entity.
const RETRACTED: i8 = 0;
const EXPANDING: i8 = 1;
const EXPANDED: i8 = 2;
const RETRACTING: i8 = 3;

struct Piston {
    front: Face,
    is_sticky: bool,
    facing: &'static str,
}

/// A piston going out or back: started in tick `started`, a stage a tick.
struct Move {
    layer: Entity,
    pos: BlockPos,
    extends: bool,
    started: u64,
    stages_done: u64,
    /// The cells whose blocks it takes along, farthest first, and those whose blocks it breaks.
    taken: Vec<BlockPos>,
    broken: Vec<BlockPos>,
    /// Where each block taken along is, what it is, and its block entity.
    carried: Vec<(BlockPos, u32, Option<Value>)>,
    /// Whether it is powered by now.
    is_powered: bool,
}

#[derive(Resource, Default)]
pub(crate) struct Pistons(Vec<Move>);

fn block_name(world: &World, layer: Entity, [x, y, z]: BlockPos) -> Option<(u32, &'static str, &'static str)> {
    let level = level(world, layer).filter(|level| level.in_height(y) && level.is_loaded(send::column([x, y, z])))?;
    let id = level.block(x, y, z);
    let state = level.registry().get(id)?;
    Some((id, state.name.strip_prefix("minecraft:").unwrap_or(state.name), state.properties))
}

fn piston_at(world: &World, layer: Entity, pos: BlockPos) -> Option<Piston> {
    let (_, name, properties) = block_name(world, layer, pos)?;
    let facing = properties.split(',').find_map(|pair| pair.strip_prefix("facing_direction="))?;
    let piston = |is_sticky| Some(Piston { front: pushes(facing)?, is_sticky, facing });
    match name {
        "piston" => piston(false),
        "sticky_piston" => piston(true),
        _ => None,
    }
}

fn is_extended(world: &World, layer: Entity, pos: BlockPos, piston: &Piston) -> bool {
    block_name(world, layer, piston.front.neighbour(pos)).is_some_and(|(_, name, properties)| name.ends_with("piston_arm_collision") && properties == format!("facing_direction={}", piston.facing))
}

fn back(front: Face) -> Face {
    PUSHES[PUSHES.iter().position(|face| *face == front).unwrap_or(0) ^ 1]
}

/// The piston's block entity in `state`, naming the cells of the blocks it takes and breaks.
fn arm(pos: BlockPos, state: i8, is_sticky: bool, taken: &[BlockPos], broken: &[BlockPos]) -> Value {
    let cells = |cells: &[BlockPos]| Value::List(List { tag: if cells.is_empty() { 0 } else { 3 }, items: cells.iter().flatten().map(|at| Value::Int(*at)).collect() });
    let progress = if matches!(state, EXPANDED | RETRACTING) { 1.0 } else { 0.0 };
    let Value::Compound(mut fields) = new_block_entity("PistonArm", pos) else { unreachable!("a block entity is a compound") };
    fields.extend([
        (Str::from("AttachedBlocks"), cells(taken)),
        (Str::from("BreakBlocks"), cells(broken)),
        (Str::from("LastProgress"), Value::Float(progress)),
        (Str::from("NewState"), Value::Byte(state)),
        (Str::from("Progress"), Value::Float(progress)),
        (Str::from("State"), Value::Byte(state)),
        (Str::from("Sticky"), Value::Byte(i8::from(is_sticky))),
        (Str::from("isMovable"), Value::Byte(i8::from(state == RETRACTED))),
    ]);
    Value::Compound(fields)
}

/// Gives a piston that was just put at `pos` its block entity, drawn back.
pub(crate) fn furnish(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(piston) = piston_at(world, layer, pos) else { return };
    if level(world, layer).is_some_and(|level| level.block_entity(pos).is_none()) {
        send::keep_entity(world, layer, pos, &arm(pos, RETRACTED, piston.is_sticky, &[], &[]));
    }
}

/// Sets the piston at `pos` going out or back as its power says, or notes the power for when it
/// is done going.
pub(crate) fn power(world: &mut World, layer: Entity, pos: BlockPos, is_powered: bool) {
    let Some(piston) = piston_at(world, layer, pos) else { return };
    if let Some(going) = world.get_resource_or_insert_with(Pistons::default).0.iter_mut().find(|going| (going.layer, going.pos) == (layer, pos)) {
        going.is_powered = is_powered;
        return;
    }
    if is_extended(world, layer, pos, &piston) == is_powered {
        return;
    }
    let (taken, broken) = match is_powered {
        true => match plan::pushed(world, layer, pos, piston.front) {
            Some(lists) => lists,
            None => return,
        },
        false if piston.is_sticky => plan::drawn(world, layer, pos, back(piston.front)),
        false => Default::default(),
    };
    send::keep_entity(world, layer, pos, &arm(pos, if is_powered { EXPANDING } else { RETRACTING }, piston.is_sticky, &taken, &broken));
    let started = world.resource::<Tick>().0;
    world.resource_mut::<Pistons>().0.push(Move { layer, pos, extends: is_powered, started, stages_done: 1, taken, broken, carried: Vec::new(), is_powered });
}

/// Takes the block at `from` to `to` as a moving block, for the piston at `piston`.
fn carry(world: &mut World, layer: Entity, from: BlockPos, to: BlockPos, piston: BlockPos, extends: bool) -> Option<(BlockPos, u32, Option<Value>)> {
    let level = level(world, layer)?;
    let (registry, id) = (level.registry(), level.block(from[0], from[1], from[2]));
    let entity = level.block_entity(from).map(|entity| entity.nbt.clone());
    let (moving, air) = (registry.find(MOVING, "")?, registry.air_id());
    let Value::Compound(mut fields) = new_block_entity("MovingBlock", to) else { return None };
    fields.extend([
        (Str::from("expanding"), Value::Byte(i8::from(extends))),
        (Str::from("movingBlock"), block_of_hash(registry, registry.network_hash(id))?),
        (Str::from("movingBlockExtra"), block_of_hash(registry, registry.network_hash(air))?),
        (Str::from("pistonPosX"), Value::Int(piston[0])),
        (Str::from("pistonPosY"), Value::Int(piston[1])),
        (Str::from("pistonPosZ"), Value::Int(piston[2])),
    ]);
    fields.extend(entity.clone().map(|entity| (Str::from("movingEntity"), entity)));
    send::set(world, layer, from, air);
    send::set(world, layer, to, moving);
    send::keep_entity(world, layer, to, &Value::Compound(fields));
    Some((to, id, entity))
}

/// The block entity `entity` at another place.
fn at_place(entity: Value, pos: BlockPos) -> Value {
    let Value::Compound(fields) = entity else { return entity };
    let placed = |(key, value): (Str, Value)| match ["x", "y", "z"].iter().position(|axis| key == Str::from(*axis)) {
        Some(axis) => (key, Value::Int(pos[axis])),
        None => (key, value),
    };
    Value::Compound(fields.into_iter().map(placed).collect())
}

/// How far its blocks are in the tick a piston starts out; they are there a tick later.
const HALFWAY: f32 = 0.5;

/// The cells whose blocks a piston going out pushes, its own for its arm first, then the nearest.
fn pushed(going: &Move) -> Vec<BlockPos> {
    std::iter::once(going.pos).chain(going.taken.iter().rev().copied()).collect()
}

/// One tick of a piston's going: the blocks go, then it is heard and is where it was going, then
/// the blocks are blocks again.
fn stage(world: &mut World, going: &mut Move) -> Option<()> {
    let (layer, pos) = (going.layer, going.pos);
    let piston = piston_at(world, layer, pos)?;
    let front = piston.front.neighbour(pos);
    match going.stages_done {
        1 => {
            going.broken.iter().for_each(|at| updates::pop(world, layer, *at));
            if !going.extends {
                let air = level(world, layer)?.registry().air_id();
                send::set(world, layer, front, air);
            }
            let step = if going.extends { piston.front } else { back(piston.front) };
            going.carried = going.taken.clone().into_iter().filter_map(|from| carry(world, layer, from, step.neighbour(from), pos, going.extends)).collect();
            if going.extends {
                let name = if piston.is_sticky { "minecraft:sticky_piston_arm_collision" } else { "minecraft:piston_arm_collision" };
                let head = level(world, layer)?.registry().find(name, &format!("facing_direction={}", piston.facing))?;
                send::set(world, layer, front, head);
                // Only the arm pushes halfway: what is behind a moved block goes once, at the end (bds-shove.txt).
                shove::shove(world, layer, &[pos], piston.front, HALFWAY);
            }
        }
        2 => {
            let [x, y, z] = pos.map(|c| c as f32);
            send::to_viewers(world, layer, pos, &[send::noise(if going.extends { "piston.out" } else { "piston.in" }, Vec3f { x, y, z }, -1)]);
            send::keep_entity(world, layer, pos, &arm(pos, if going.extends { EXPANDED } else { RETRACTED }, piston.is_sticky, &[], &[]));
            if going.extends {
                shove::shove(world, layer, &pushed(going), piston.front, 1.0);
            }
        }
        _ => {
            for (at, id, entity) in std::mem::take(&mut going.carried) {
                send::set(world, layer, at, id);
                if let Some(entity) = entity {
                    send::keep_entity(world, layer, at, &at_place(entity, at));
                }
                updates::settle(world, layer, at);
            }
            going.taken.iter().chain(&going.broken).chain([&front]).for_each(|at| updates::settle(world, layer, *at));
        }
    }
    Some(())
}

/// Moves on every piston that is going, and turns back those whose power changed meanwhile.
pub(crate) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let Some(mut pistons) = world.get_resource_mut::<Pistons>().filter(|pistons| !pistons.0.is_empty()) else { return };
    let mut going = std::mem::take(&mut pistons.0);
    let mut done = Vec::new();
    for mut one in std::mem::take(&mut going) {
        while one.stages_done < 4 && now >= one.started + one.stages_done {
            let staged = stage(world, &mut one);
            one.stages_done = if staged.is_some() { one.stages_done + 1 } else { 4 };
        }
        if one.stages_done < 4 { going.push(one) } else { done.push(one) }
    }
    world.resource_mut::<Pistons>().0.extend(going);
    for one in done.into_iter().filter(|one| one.is_powered != one.extends) {
        power(world, one.layer, one.pos, one.is_powered);
    }
}
