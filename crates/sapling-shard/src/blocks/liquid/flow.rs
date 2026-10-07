//! One move of a liquid: BDS `LiquidBlock::_tick` (docs/research/water.md "Pseudo-code").

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};
use sapling_data::LiquidReaction;

use super::super::loot::{self, Breaker};
use super::super::{Dice, level};
use super::{Cell, Liquid, cell, disturb, faces, harden, is_hot, schedule, set};

const FARTHEST: u8 = 7;
const FALLING: u8 = 8;
/// The cost of a way with no drop in reach.
const NO_DROP: u8 = u8::MAX;
/// In the game's order; a side and its opposite differ in the lowest bit of their index.
const SIDES: [Face; 4] = [Face::West, Face::East, Face::North, Face::South];
/// Lava that thins out waits this many times its delay, three times in four.
const SLOWED: u64 = 4;

fn depth(world: &World, layer: Entity, pos: BlockPos, liquid: Liquid) -> Option<u8> {
    match cell(world, layer, pos) {
        Cell::Liquid(found, depth) if found == liquid => Some(depth),
        _ => None,
    }
}

fn is_solid(world: &World, layer: Entity, [x, y, z]: BlockPos) -> bool {
    level(world, layer).is_some_and(|level| level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_solid()))
}

/// Whether `liquid` at `from` cannot run into the cell on `toward`: that cell keeps it out (lava
/// a waterlogged block too), or, for water, a face between them is closed (faces.rs).
fn stopped(world: &World, layer: Entity, from: BlockPos, toward: Face, liquid: Liquid) -> bool {
    let to = toward.neighbour(from);
    cell(world, layer, to).stops(liquid) || liquid == Liquid::Lava && is_solid(world, layer, to) || liquid == Liquid::Water && !faces::passes(world, layer, from, toward)
}

/// What the flowing liquid at `pos` should be, going by what is beside and above it: `None` when
/// nothing feeds it any more. A neighbour feeds only through an open face of its own.
fn fed(world: &World, layer: Entity, pos: BlockPos, liquid: Liquid, drop_off: u8) -> Option<u8> {
    let feeds = |side: &&Face| liquid == Liquid::Lava || faces::open(world, layer, side.neighbour(pos), faces::opposite(**side));
    let beside: Vec<u8> = SIDES.iter().filter(feeds).filter_map(|side| depth(world, layer, side.neighbour(pos), liquid)).collect();
    let (above, below) = (depth(world, layer, Face::Up.neighbour(pos), liquid), Face::Down.neighbour(pos));
    let sources = beside.iter().filter(|depth| **depth == 0).count();
    // Two sources beside, or one beside and one above, over a floor or a source: a pool refills.
    let refills = sources >= 2 || sources == 1 && above == Some(0);
    if liquid == Liquid::Water && refills && (is_solid(world, layer, below) || depth(world, layer, below, liquid) == Some(0)) {
        return Some(0);
    }
    if let Some(above) = above {
        return Some(if above >= FALLING { above } else { above + FALLING });
    }
    // A falling liquid beside feeds like a source.
    let nearest = beside.iter().map(|depth| if *depth >= FALLING { 0 } else { *depth }).min()?;
    Some(nearest + drop_off).filter(|depth| *depth <= FARTHEST)
}

/// Whether `liquid` that ran into `to` could go down there.
fn drops_at(world: &World, layer: Entity, to: BlockPos, liquid: Liquid) -> bool {
    !stopped(world, layer, to, Face::Down, liquid)
}

/// Whether `liquid` at `from` can run into the cell on `toward`: not through a block or a closed
/// face, and not into its own source.
fn takes_a_stream(world: &World, layer: Entity, from: BlockPos, toward: Face, liquid: Liquid) -> bool {
    !stopped(world, layer, from, toward, liquid) && depth(world, layer, toward.neighbour(from), liquid) != Some(0)
}

/// How many steps from `pos` the nearest drop is, having come `came` (an index into [`SIDES`])
/// and being `step` steps out; [`NO_DROP`] when there is none within the liquid's search.
fn slope(world: &World, layer: Entity, pos: BlockPos, liquid: Liquid, step: u8, came: usize) -> u8 {
    let mut best = NO_DROP;
    for (way, side) in SIDES.iter().enumerate().filter(|(way, _)| *way != came ^ 1) {
        let next = side.neighbour(pos);
        if !takes_a_stream(world, layer, pos, *side, liquid) {
            continue;
        }
        if drops_at(world, layer, next, liquid) {
            return step;
        }
        if step < liquid.search() {
            best = best.min(slope(world, layer, next, liquid, step + 1, way));
        }
    }
    best
}

/// Lets `liquid` of `depth` into the cell, unless it holds the same liquid or lava, or keeps it
/// out (BDS `_trySpreadTo`). Water leaves a Popped block's loot and takes in a Noreaction one
/// beside itself; lava burns what it takes the place of, and boils water away.
fn spread_to(world: &mut World, layer: Entity, from: BlockPos, toward: Face, liquid: Liquid, depth: u8) {
    if stopped(world, layer, from, toward, liquid) {
        return;
    }
    let pos = toward.neighbour(from);
    match (cell(world, layer, pos), liquid) {
        (Cell::Liquid(Liquid::Lava, _), _) | (Cell::Liquid(Liquid::Water, _), Liquid::Water) | (Cell::Unknown, _) => return,
        (Cell::Liquid(..) | Cell::Block(..), Liquid::Lava) => harden::fizz(world, layer, pos),
        (Cell::Block(LiquidReaction::Popped, _), Liquid::Water) => {
            let named = level(world, layer).and_then(|level| level.registry().get(level.block(pos[0], pos[1], pos[2]))).map(|state| (state.name, state.properties));
            if let Some((name, properties)) = named {
                let left = loot::thrown(world, name, properties, &Breaker::UNAIDED);
                loot::spill(world, layer, pos, &left);
            }
        }
        (Cell::Air | Cell::Block(..), _) => {}
    }
    set(world, layer, pos, liquid, Some(depth));
}

fn falling(depth: u8) -> u8 {
    if depth >= FALLING { depth } else { depth + FALLING }
}

/// Moves the liquid at `pos` one step: it takes the depth its neighbours give it or dries up,
/// then falls, or on a floor runs towards the nearest drop, or every open way when there is none.
pub(super) fn flow(world: &mut World, layer: Entity, pos: BlockPos) {
    let Cell::Liquid(liquid, mut depth) = cell(world, layer, pos) else { return };
    let hot = is_hot(world, layer);
    let drop_off = liquid.drop_off(hot);
    let mut changed = false;
    if depth != 0 {
        let Some(now) = fed(world, layer, pos, liquid, drop_off) else {
            set(world, layer, pos, liquid, None);
            return;
        };
        if now != depth {
            // Lava that thins out lingers.
            if liquid == Liquid::Lava && depth < FALLING && now < FALLING && depth < now && world.get_resource_or_insert_with(Dice::default).roll(4) != 0 {
                schedule(world, layer, pos, liquid.delay(hot) * SLOWED);
            }
            set(world, layer, pos, liquid, Some(now));
            (depth, changed) = (now, true);
        }
    }
    if !changed && harden::meets_water(world, layer, pos) {
        return disturb(world, layer, pos);
    }
    let below = Face::Down.neighbour(pos);
    if !stopped(world, layer, pos, Face::Down, liquid) {
        match cell(world, layer, below) {
            Cell::Liquid(Liquid::Water, _) if liquid == Liquid::Lava => {
                if harden::turn(world, layer, below, harden::STONE) {
                    disturb(world, layer, below);
                }
                return;
            }
            // Flowing over a liquid goes no way but down; a source still spreads.
            Cell::Liquid(..) if depth != 0 => return,
            Cell::Liquid(..) => {}
            _ => return spread_to(world, layer, pos, Face::Down, liquid, falling(depth)),
        }
    }
    let next = if depth >= FALLING { 1 } else { depth + drop_off };
    if next > FARTHEST {
        return;
    }
    // A thin stream looks less far for a drop.
    let first_step = if (5..=FARTHEST).contains(&depth) { depth - 3 } else { 1 };
    let costs = [0, 1, 2, 3].map(|way| {
        let to = SIDES[way].neighbour(pos);
        match takes_a_stream(world, layer, pos, SIDES[way], liquid) {
            false => None,
            true if drops_at(world, layer, to, liquid) => Some(0),
            true => Some(slope(world, layer, to, liquid, first_step, way)),
        }
    });
    let Some(cheapest) = costs.iter().flatten().min().copied() else { return };
    for way in (0..SIDES.len()).filter(|way| costs[*way] == Some(cheapest)) {
        spread_to(world, layer, pos, SIDES[way], liquid, next);
    }
}
