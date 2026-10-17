//! Scaffolding: each block knows how far it is from what holds it up, and one seven away comes
//! down, falling if it was only just put there and broken if it stood
//! (docs/research/block-uses.md, "Scaffolding").
// TODO: scaffolding a world folder holds is not looked at until something by it is written.

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::{falling, level, send, updates, with_property};
use crate::components::Tick;

const SCAFFOLDING: &str = "minecraft:scaffolding";
const STABILITY: &str = "stability";
const CHECKED: &str = "stability_check";
/// This far from its hold, scaffolding does not stay.
const FAR: u8 = 7;
/// Ticks from a block being put to its first look, and from a change by it to its next.
const FIRST_LOOK: u64 = 2;
const NEXT_LOOK: u64 = 1;
const SIDES: [[i32; 2]; 4] = [[1, 0], [-1, 0], [0, 1], [0, -1]];

type Level = sapling_world::World;

/// Scaffolding that is to look at its hold: when, and where.
#[derive(Resource, Default)]
pub(crate) struct Scaffolds(Vec<(u64, Entity, BlockPos)>);

/// The scaffolding at `pos`: its state, how far it says it is from its hold, and whether it
/// has looked yet.
fn scaffolding(level: &Level, [x, y, z]: BlockPos) -> Option<(u32, u8, bool)> {
    let id = level.in_height(y).then(|| level.block(x, y, z))?;
    let state = level.registry().get(id).filter(|state| state.name == SCAFFOLDING)?;
    let number = |key: &str| state.properties.split(',').filter_map(|pair| pair.split_once('=')).find(|(name, _)| *name == key).and_then(|(_, value)| value.parse::<u8>().ok());
    Some((id, number(STABILITY)?, number(CHECKED)? == 1))
}

/// How far the cell `pos` is from a hold: none over a solid block, as far as the scaffolding
/// under it, and one more than the nearest scaffolding beside it.
fn distance(level: &Level, [x, y, z]: BlockPos) -> u8 {
    let under = [x, y - 1, z];
    let stands = level.in_height(y - 1) && level.registry().get(level.block(x, y - 1, z)).is_some_and(|state| state.is_solid());
    let below = scaffolding(level, under).map_or(if stands { 0 } else { FAR }, |(_, far, _)| far);
    let beside = SIDES.iter().filter_map(|[dx, dz]| scaffolding(level, [x + dx, y, z + dz])).map(|(_, far, _)| far + 1).min().unwrap_or(FAR);
    below.min(beside).min(FAR)
}

/// A block was written at `pos`: scaffolding there, over it and beside it looks at its hold.
pub(super) fn written(world: &mut World, layer: Entity, [x, y, z]: BlockPos) {
    let Some(level) = level(world, layer) else { return };
    let cells = [[x, y, z], [x, y + 1, z]].into_iter().chain(SIDES.map(|[dx, dz]| [x + dx, y, z + dz]));
    let looks: Vec<(BlockPos, u64)> = cells.filter_map(|cell| scaffolding(level, cell).map(|(_, _, looked)| (cell, if looked { NEXT_LOOK } else { FIRST_LOOK }))).collect();
    if looks.is_empty() {
        return;
    }
    let now = world.resource::<Tick>().0;
    let mut due = world.get_resource_or_insert_with(Scaffolds::default);
    for (cell, wait) in looks {
        if !due.0.iter().any(|(_, in_layer, at)| (*in_layer, *at) == (layer, cell)) {
            due.0.push((now + wait, layer, cell));
        }
    }
}

/// Has the scaffolding that is due look at its hold.
pub(crate) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let Some(mut due) = world.get_resource_mut::<Scaffolds>().filter(|due| !due.0.is_empty()) else { return };
    let (looking, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut due.0).into_iter().partition(|(when, ..)| *when <= now);
    due.0 = waiting;
    for (_, layer, pos) in looking {
        let Some(level) = level(world, layer) else { continue };
        let Some((id, said, looked)) = scaffolding(level, pos) else { continue };
        let far = distance(level, pos);
        if far == FAR {
            // What was just put there falls; what stood breaks, as BDS's row did with its foot gone.
            if looked {
                updates::pop(world, layer, pos);
            } else {
                falling::let_fall(world, layer, pos);
            }
        } else if (said, looked) != (far, true) {
            let registry = level.registry();
            let settled = with_property(registry, id, STABILITY, &far.to_string()).and_then(|id| with_property(registry, id, CHECKED, "1"));
            if let Some(settled) = settled {
                send::set(world, layer, pos, settled);
            }
        }
    }
}
