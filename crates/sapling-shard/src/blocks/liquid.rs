//! Water and lava that find room, by the game's own rule: `LiquidBlock::_tick` of the Bedrock
//! server as decompiled (docs/research/water.md has the rule and where it was read). A cell's
//! `liquid_depth` is 0 for a source, 1 to 7 for how far it has run, and 8 to 15 for a falling
//! liquid, which keeps the depth of the stream it fell from. The rule is flow.rs; lava meeting
//! water hardens (harden.rs).
// Lava's random tick sets fire (fire.rs `kindled_by_lava`); which blocks it takes is lava.rs.
// TODO: `ctx.set_block` starts nothing.

mod faces;
mod flow;
mod harden;
mod lava;

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};
use sapling_data::LiquidReaction;

use super::queued::{self, Ticked};
use super::{level, send, waterlog};

/// Cells looked at for one change, hardening included.
const MAX_DISTURBED: usize = 4096;
const NETHER: i32 = 1;
const BUBBLE_COLUMN: &str = "minecraft:bubble_column";

pub(super) use crate::body::Liquid;

impl Liquid {
    /// Every cell a liquid writes is of this block, never the `flowing_` one.
    pub(super) fn name(self) -> &'static str {
        match self {
            Liquid::Water => "minecraft:water",
            Liquid::Lava => "minecraft:lava",
        }
    }

    /// Ticks between a change and the liquid next to it moving; `hot`: in an ultrawarm world.
    fn delay(self, hot: bool) -> u64 {
        match self {
            Liquid::Water => 5,
            Liquid::Lava if hot => 10,
            Liquid::Lava => 30,
        }
    }

    /// How much shallower a stream gets with each cell it runs.
    fn drop_off(self, hot: bool) -> u8 {
        if self == Liquid::Lava && !hot { 2 } else { 1 }
    }

    /// How many steps away a drop is looked for, counting from 1.
    fn search(self) -> u8 {
        if self == Liquid::Water { 4 } else { 2 }
    }
}

/// What a cell is to a liquid.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Cell {
    Liquid(Liquid, u8),
    Air,
    /// A block, by what water does to it and whether it keeps lava out (lava.rs).
    Block(LiquidReaction, bool),
    /// Outside the world's height or not loaded.
    Unknown,
}

impl Cell {
    /// Whether `liquid` cannot run into the cell (BDS `LiquidBlock::_isLiquidBlocking`): water by
    /// the block's reaction, lava by the block's type (`isLavaBlocking`).
    pub(super) fn stops(self, liquid: Liquid) -> bool {
        match (self, liquid) {
            (Cell::Unknown, _) => true,
            (Cell::Liquid(..) | Cell::Air, _) => false,
            (Cell::Block(reaction, _), Liquid::Water) => reaction == LiquidReaction::Blocking,
            (Cell::Block(_, keeps_lava_out), Liquid::Lava) => keeps_lava_out,
        }
    }
}

/// What the cell holds: the water a waterlogged block keeps in layer 1 counts as the cell's
/// (BDS `BlockSource::getLiquidBlock`).
pub(super) fn cell(world: &World, layer: Entity, [x, y, z]: BlockPos) -> Cell {
    let Some(level) = level(world, layer).filter(|level| level.in_height(y) && level.is_loaded(send::column([x, y, z]))) else { return Cell::Unknown };
    let registry = level.registry();
    let liquid = |id: u32| registry.get(id).and_then(|state| Some(Cell::Liquid(Liquid::of(state.name)?, state.liquid_depth)));
    let (block, held) = (level.block(x, y, z), level.liquid(x, y, z));
    if let Some(liquid) = liquid(held).or_else(|| liquid(block)) {
        return liquid;
    }
    match registry.get(block) {
        Some(state) if state.is_air() => Cell::Air,
        Some(state) => Cell::Block(waterlog::reaction(registry, block), lava::keeps_out(state.name)),
        None => Cell::Unknown,
    }
}

/// Whether the layer's world is ultrawarm: the nether.
pub(super) fn is_hot(world: &World, layer: Entity) -> bool {
    level(world, layer).is_some_and(|level| level.dimension() == NETHER)
}

/// Queues `liquid` at `pos` to move in `delay` ticks, in its column's block tick queue (queued.rs);
/// a neighbour's change does not queue it twice for the same tick (BDS `LiquidBlock::neighborChanged`),
/// anything else does.
fn schedule(world: &mut World, layer: Entity, pos: BlockPos, liquid: Liquid, delay: u64, by_neighbour: bool) {
    let due = queued::clock(world, layer, pos) + delay;
    let ticked = Ticked::Flow(liquid);
    if !by_neighbour || !queued::has(world, layer, pos, Some(due), ticked) {
        queued::add(world, layer, pos, due, ticked);
    }
}

/// Queues `liquid` at `pos` to move in `delay` ticks, as its column kept it.
pub(super) fn again(world: &mut World, layer: Entity, pos: BlockPos, liquid: Liquid, delay: u64) {
    queued::again(world, layer, pos, delay, Ticked::Flow(liquid));
}

/// After a liquid moved into or out of `pos`: the liquid there and beside it is queued to move,
/// and lava that now touches water hardens.
pub(super) fn disturb(world: &mut World, layer: Entity, pos: BlockPos) {
    wake(world, layer, pos, true);
    super::sulfur::neighbours_changed(world, layer, pos);
}

/// After a block at `pos` changed for another reason (a break, a place, a handler's write): only
/// the liquid beside it is queued, as BDS's `neighborChanged` reaches the neighbours; a liquid
/// written there itself waits for them (`setblock water` stands still). Lava hardens all the same.
pub(crate) fn neighbours_changed(world: &mut World, layer: Entity, pos: BlockPos) {
    wake(world, layer, pos, false);
}

fn wake(world: &mut World, layer: Entity, pos: BlockPos, itself: bool) {
    let hot = is_hot(world, layer);
    let mut changed = vec![pos];
    let mut looked = 0;
    while let Some(at) = changed.pop() {
        for near in Face::ALL.iter().map(|face| face.neighbour(at)).chain([at]) {
            looked += 1;
            if looked > MAX_DISTURBED {
                return;
            }
            if harden::meets_water(world, layer, near) {
                changed.push(near);
            } else if let (Cell::Liquid(liquid, _), true) = (cell(world, layer, near), itself || near != pos) {
                schedule(world, layer, near, liquid, liquid.delay(hot), true);
            }
        }
    }
}

/// Makes the cell `liquid` of `depth`, or empties it: in layer 1 beside a block that holds water
/// or lets it in (Noreaction), else in layer 0 over what was there. Queues it and what is around
/// it to move.
fn set(world: &mut World, layer: Entity, pos: BlockPos, liquid: Liquid, depth: Option<u8>) -> bool {
    let Some(level) = level(world, layer) else { return false };
    let registry = level.registry();
    let [x, y, z] = pos;
    let (block, held) = (level.block(x, y, z), level.liquid(x, y, z));
    let beside_block = registry.get(block).is_some_and(|state| !state.is_air() && !state.is_liquid());
    let holds = registry.get(held).is_some_and(|state| state.is_liquid());
    let in_layer_zero = !beside_block || liquid == Liquid::Lava || !holds && waterlog::reaction(registry, block) != LiquidReaction::Noreaction;
    let Some(id) = depth.map_or(Some(registry.air_id()), |depth| registry.find(liquid.name(), &format!("liquid_depth={depth}"))) else { return false };
    let written = if in_layer_zero { send::set(world, layer, pos, id) } else { send::set_liquid(world, layer, pos, id) };
    // A block that cannot stand in the water run in beside it breaks, the water taking its cell
    // (cave and nether vines, research/captures/bds-plantpop2.txt).
    if written && !in_layer_zero && super::level(world, layer).is_some_and(|level| !super::ground::may_stand(level, pos, block)) {
        super::updates::pop(world, layer, pos);
    }
    if written {
        disturb(world, layer, pos);
    }
    written
}

/// Pours a source of `liquid` into the cell, into layer 1 beside the block there when
/// `waterlogs`, and sets it flowing (BDS `BucketItem::_emptyBucket`).
pub(super) fn pour(world: &mut World, layer: Entity, pos: BlockPos, liquid: Liquid, waterlogs: bool) -> bool {
    let Some(id) = level(world, layer).and_then(|level| level.registry().find(liquid.name(), "liquid_depth=0")) else { return false };
    let poured = if waterlogs { send::set_liquid(world, layer, pos, id) } else { send::set(world, layer, pos, id) };
    if poured && waterlogs {
        send::log_flowing(world, layer, pos, id);
    }
    if poured {
        disturb(world, layer, pos);
    }
    poured
}

/// Takes the liquid out of the cell: the water a waterlogged block keeps, else the cell's own,
/// with any bubble column in it.
pub(super) fn take(world: &mut World, layer: Entity, [x, y, z]: BlockPos) {
    let Some(level) = level(world, layer) else { return };
    let registry = level.registry();
    let (air, holds) = (registry.air_id(), registry.get(level.liquid(x, y, z)).is_some_and(|state| state.is_liquid()));
    let is_column = registry.get(level.block(x, y, z)).is_some_and(|state| state.name == BUBBLE_COLUMN);
    if holds {
        send::clear_liquid(world, layer, [x, y, z]);
    }
    if !holds || is_column {
        send::set(world, layer, [x, y, z], air);
    }
    disturb(world, layer, [x, y, z]);
}

/// The queued tick of `liquid` at `pos` (queued.rs): it moves, unless the cell's liquid is no longer
/// the one queued or the world has `sapling:liquidFlow` off (its liquids stand as they are).
pub(super) fn ticked(world: &mut World, layer: Entity, pos: BlockPos, liquid: Liquid) {
    let holds = matches!(cell(world, layer, pos), Cell::Liquid(now_there, _) if now_there == liquid);
    if holds && crate::rules::on(world, layer, sapling_data::LIQUID_FLOW) {
        flow::flow(world, layer, pos);
    }
}
