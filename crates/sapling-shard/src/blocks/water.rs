//! Water that finds room, by the game's own rule: `LiquidBlock::_tick` of the Bedrock server as
//! decompiled (docs/DESIGN.md "Water" has the rule and where it was read). A cell's
//! `liquid_depth` is 0 for a source, 1 to 7 for how far it has run, and 8 to 15 for falling
//! water, which keeps the depth of the stream it fell from.
// TODO: layer 0 only: waterlogged blocks neither feed nor take water; lava does not flow or
// harden; what gives way is place.rs's list, all of it leaving its item, where the game has a
// reaction per block; no buckets; `ctx.set_block` starts nothing.

use std::collections::{HashSet, VecDeque};

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};

use super::loot::{self, Breaker};
use super::place::gives_way;
use super::{level, send};
use crate::components::Tick;

const WATER: &str = "minecraft:water";
const FLOWING_WATER: &str = "minecraft:flowing_water";
/// Ticks between a change and the water next to it moving.
const DELAY: u64 = 5;
/// Cells moved in one tick; the rest wait for the next.
const MAX_PER_TICK: usize = 512;
const FARTHEST: u8 = 7;
const FALLING: u8 = 8;
/// How many steps away a drop is looked for, counting from 1.
const SEARCH: u8 = 4;
/// The cost of a way with no drop in reach.
const NO_DROP: u8 = u8::MAX;
/// In the game's order; a side and its opposite differ in the lowest bit of their index.
const SIDES: [Face; 4] = [Face::West, Face::East, Face::North, Face::South];

/// The cells whose water is due to move.
#[derive(Resource, Default)]
pub(crate) struct Flows {
    due: VecDeque<(u64, Entity, BlockPos)>,
    queued: HashSet<(Entity, BlockPos)>,
}

/// What a cell is to water.
#[derive(Clone, Copy, PartialEq)]
enum Cell {
    Water(u8),
    /// Air, or (`true`) a plant that water washes away.
    Open(bool),
    Blocked,
}

fn cell(world: &World, layer: Entity, [x, y, z]: BlockPos) -> Cell {
    let Some(level) = level(world, layer).filter(|level| level.in_height(y) && level.is_loaded(send::column([x, y, z]))) else { return Cell::Blocked };
    let (registry, id) = (level.registry(), level.block(x, y, z));
    let Some(state) = registry.get(id) else { return Cell::Blocked };
    let depth = || state.properties.split(',').find_map(|pair| pair.strip_prefix("liquid_depth=")).and_then(|depth| depth.parse().ok());
    match state.name {
        WATER | FLOWING_WATER => Cell::Water(depth().unwrap_or(0)),
        _ if !state.is_liquid() && gives_way(registry, id) => Cell::Open(!state.is_air()),
        _ => Cell::Blocked,
    }
}

fn depth(world: &World, layer: Entity, pos: BlockPos) -> Option<u8> {
    if let Cell::Water(depth) = cell(world, layer, pos) { Some(depth) } else { None }
}

fn is_solid(world: &World, layer: Entity, [x, y, z]: BlockPos) -> bool {
    level(world, layer).is_some_and(|level| level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_solid()))
}

/// Queues the water at `pos` and around it to move, after a change there.
pub(super) fn disturb(world: &mut World, layer: Entity, pos: BlockPos) {
    let wet: Vec<BlockPos> = Face::ALL.iter().map(|face| face.neighbour(pos)).chain([pos]).filter(|at| depth(world, layer, *at).is_some()).collect();
    let due = world.resource::<Tick>().0 + DELAY;
    let mut flows = world.get_resource_or_insert_with(Flows::default);
    for at in wet {
        if flows.queued.insert((layer, at)) {
            flows.due.push_back((due, layer, at));
        }
    }
}

/// Makes the cell water of `depth`, or air, and queues it and the water around it.
fn set(world: &mut World, layer: Entity, pos: BlockPos, depth: Option<u8>) {
    let Some(registry) = level(world, layer).map(|level| level.registry()) else { return };
    let id = match depth {
        Some(depth) => registry.find(WATER, &format!("liquid_depth={depth}")),
        None => Some(registry.air_id()),
    };
    if id.is_some_and(|id| send::set(world, layer, pos, id)) {
        disturb(world, layer, pos);
    }
}

/// What the flowing water at `pos` should be, going by what is beside and above it: `None` when
/// nothing feeds it any more.
fn fed(world: &World, layer: Entity, pos: BlockPos) -> Option<u8> {
    let beside: Vec<u8> = SIDES.iter().filter_map(|side| depth(world, layer, side.neighbour(pos))).collect();
    let (above, below) = (depth(world, layer, Face::Up.neighbour(pos)), Face::Down.neighbour(pos));
    let sources = beside.iter().filter(|depth| **depth == 0).count();
    // Two sources beside, or one beside and one above, over a floor or a source: a pool refills.
    if (sources >= 2 || sources == 1 && above == Some(0)) && (is_solid(world, layer, below) || depth(world, layer, below) == Some(0)) {
        return Some(0);
    }
    if let Some(above) = above {
        return Some(if above >= FALLING { above } else { above + FALLING });
    }
    // Falling water beside feeds like a source.
    let nearest = beside.iter().map(|depth| if *depth >= FALLING { 0 } else { *depth }).min()?;
    Some(nearest + 1).filter(|depth| *depth <= FARTHEST)
}

/// Whether water that ran into `to` could go down there: into room, or onto water.
fn drops_at(world: &World, layer: Entity, to: BlockPos) -> bool {
    cell(world, layer, Face::Down.neighbour(to)) != Cell::Blocked
}

/// Whether water can run into the cell: not through a block, and not into a source.
fn takes_a_stream(world: &World, layer: Entity, pos: BlockPos) -> bool {
    !matches!(cell(world, layer, pos), Cell::Blocked | Cell::Water(0))
}

/// How many steps from `pos` the nearest drop is, having come `came` (an index into [`SIDES`])
/// and being `step` steps out; [`NO_DROP`] when there is none within [`SEARCH`].
fn slope(world: &World, layer: Entity, pos: BlockPos, step: u8, came: usize) -> u8 {
    let mut best = NO_DROP;
    for (way, side) in SIDES.iter().enumerate().filter(|(way, _)| *way != came ^ 1) {
        let next = side.neighbour(pos);
        if !takes_a_stream(world, layer, next) {
            continue;
        }
        if drops_at(world, layer, next) {
            return step;
        }
        if step < SEARCH {
            best = best.min(slope(world, layer, next, step + 1, way));
        }
    }
    best
}

/// Lets water of `depth` into the cell, unless it is water already or blocked; a plant there
/// is washed away and leaves its item.
fn spread_to(world: &mut World, layer: Entity, pos: BlockPos, depth: u8) {
    let Cell::Open(washes_away) = cell(world, layer, pos) else { return };
    if washes_away {
        let named = level(world, layer).and_then(|level| level.registry().get(level.block(pos[0], pos[1], pos[2]))).map(|state| (state.name, state.properties));
        if let Some((name, properties)) = named {
            let left = loot::thrown(world, name, properties, &Breaker::UNAIDED);
            loot::spill(world, layer, pos, &left);
        }
    }
    set(world, layer, pos, Some(depth));
}

/// Moves the water at `pos` one step: it takes the depth its neighbours give it or dries up,
/// then falls, or on a floor runs towards the nearest drop, or every open way when there is none.
fn flow(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(mut depth) = depth(world, layer, pos) else { return };
    if depth != 0 {
        let Some(now) = fed(world, layer, pos) else { return set(world, layer, pos, None) };
        if now != depth {
            set(world, layer, pos, Some(now));
            depth = now;
        }
    }
    let below = Face::Down.neighbour(pos);
    match cell(world, layer, below) {
        Cell::Open(_) => return spread_to(world, layer, below, if depth >= FALLING { depth } else { depth + FALLING }),
        // Flowing water over water goes no way but down; a source still spreads.
        Cell::Water(_) if depth != 0 => return,
        Cell::Water(_) | Cell::Blocked => {}
    }
    let next = if depth >= FALLING { 1 } else { depth + 1 };
    if next > FARTHEST {
        return;
    }
    // A thin stream looks less far for a drop.
    let first_step = if (5..=FARTHEST).contains(&depth) { depth - 3 } else { 1 };
    let costs = [0, 1, 2, 3].map(|way| {
        let to = SIDES[way].neighbour(pos);
        match takes_a_stream(world, layer, to) {
            false => None,
            true if drops_at(world, layer, to) => Some(0),
            true => Some(slope(world, layer, to, first_step, way)),
        }
    });
    let Some(cheapest) = costs.iter().flatten().min().copied() else { return };
    for way in (0..SIDES.len()).filter(|way| costs[*way] == Some(cheapest)) {
        spread_to(world, layer, SIDES[way].neighbour(pos), next);
    }
}

/// Moves the water that is due.
pub(crate) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let Some(mut flows) = world.get_resource_mut::<Flows>().filter(|flows| flows.due.front().is_some_and(|(due, ..)| *due <= now)) else { return };
    let ready = flows.due.iter().take(MAX_PER_TICK).take_while(|(due, ..)| *due <= now).count();
    let moving: Vec<_> = flows.due.drain(..ready).collect();
    for (_, layer, pos) in &moving {
        flows.queued.remove(&(*layer, *pos));
    }
    for (_, layer, pos) in moving {
        flow(world, layer, pos);
    }
}
