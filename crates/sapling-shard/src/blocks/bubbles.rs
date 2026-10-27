//! Bubble columns: a water source over soul sand (rising) or magma (dragging down) becomes a
//! `bubble_column` with the water kept in layer 1, and the column climbs the water a cell a tick;
//! one whose support is gone turns back to water a cell every six ticks, from the bottom up
//! (research/captures/bds-bubbles.txt). What a column does to what is in it is body/bubbles.rs.

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};
use sapling_world::BlockRegistry;

use super::queued::{self, Job};
use super::{level, send};

pub(crate) const COLUMN: &str = "minecraft:bubble_column";
const WATER: &str = "minecraft:water";
/// Ticks before a cell over a column or its support becomes one, and before one without turns back.
const FORM: u64 = 1;
const BREAK: u64 = 6;

/// What the cell over the block `below` holds as a column: `Some(drags_down)`, or none.
fn wanted(registry: &BlockRegistry, below: u32) -> Option<bool> {
    let state = registry.get(below)?;
    match state.name {
        "minecraft:soul_sand" => Some(false),
        "minecraft:magma" => Some(true),
        COLUMN => Some(state.properties.contains("drag_down=1")),
        _ => None,
    }
}

/// What the cell holds: `Some(None)` a water source, `Some(Some(drags_down))` a column, `None`
/// anything else.
fn held(registry: &BlockRegistry, id: u32) -> Option<Option<bool>> {
    let state = registry.get(id)?;
    match state.name {
        WATER if state.liquid_depth == 0 => Some(None),
        COLUMN => Some(Some(state.properties.contains("drag_down=1"))),
        _ => None,
    }
}

/// What the cell at `pos` should become, where that is not what it is: the column it carries on
/// or the water it turns back to.
fn due(world: &World, layer: Entity, [x, y, z]: BlockPos) -> Option<Option<bool>> {
    let level = level(world, layer).filter(|level| level.in_height(y) && level.in_height(y - 1) && level.is_loaded(send::column([x, y, z])))?;
    let registry = level.registry();
    let now = held(registry, level.block(x, y, z))?;
    let wanted = wanted(registry, level.block(x, y - 1, z));
    (now != wanted).then_some(wanted)
}

/// After the block at `pos` was written: it and the cell over it are queued to become or stop
/// being a column, where that is due.
pub(super) fn written(world: &mut World, layer: Entity, pos: BlockPos) {
    for cell in [pos, Face::Up.neighbour(pos)] {
        if let Some(wanted) = due(world, layer, cell) {
            queued::again(world, layer, cell, if wanted.is_some() { FORM } else { BREAK }, Job::Bubbles);
        }
    }
}

/// Makes the cell at `pos` the column its support asks for, or water again; the write queues the
/// cell over it in turn.
pub(super) fn settle(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(wanted) = due(world, layer, pos) else { return };
    let Some(registry) = level(world, layer).map(|level| level.registry()) else { return };
    let id = match wanted {
        Some(drags_down) => registry.find(COLUMN, if drags_down { "drag_down=1" } else { "drag_down=0" }),
        None => registry.find(WATER, "liquid_depth=0"),
    };
    if let Some(id) = id {
        send::set(world, layer, pos, id);
    }
}
