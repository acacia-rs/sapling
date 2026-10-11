//! Leaves decay once the wood that held them is gone (docs/research/growth.md, "Leaves"): the
//! leaves around a log or a leaf that went are marked, and a marked leaf's random tick drops it
//! if no wood is near (growth/rules.rs).
// TODO: a leaf that holds is shown with its mark cleared, where BDS's stayed marked.

use std::collections::HashSet;

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::{level, send, with_property};

const MARK: &str = "update_bit";
const KEPT: &str = "persistent_bit";
/// How far wood may be, through leaves: of BDS's row run out from a log, the four nearest held
/// and the fifth on went (research/captures/bds-leafreach.txt).
const REACH: usize = 4;
const AROUND: [[i32; 3]; 6] = [[1, 0, 0], [-1, 0, 0], [0, 1, 0], [0, -1, 0], [0, 0, 1], [0, 0, -1]];

type Level = sapling_world::World;

fn is_leaves(name: &str) -> bool {
    name.ends_with("leaves")
}

fn is_wood(name: &str) -> bool {
    name.ends_with("_log") || name.ends_with("_wood")
}

fn is_set(properties: &str, key: &str) -> bool {
    properties.split(',').filter_map(|pair| pair.split_once('=')).any(|(name, value)| name == key && value == "1")
}

fn name_at(level: &Level, [x, y, z]: BlockPos) -> Option<&str> {
    level.in_height(y).then(|| level.registry().get(level.block(x, y, z))).flatten().map(|state| state.name)
}

/// Whether these are leaves marked to see, at their random tick, whether they still hang.
pub(super) fn is_marked(name: &str, properties: &str) -> bool {
    is_leaves(name) && is_set(properties, MARK) && !is_set(properties, KEPT)
}

/// A block was written at `pos`, where `old` was: wood or leaves that went mark the leaves in
/// the 26 cells around, as BDS marked a whole cube of them when the log in it was taken out.
pub(super) fn written(world: &mut World, layer: Entity, pos: BlockPos, old: u32) {
    let Some(level) = level(world, layer) else { return };
    let registry = level.registry();
    let was = registry.get(old).map(|state| state.name).filter(|name| is_wood(name) || is_leaves(name));
    if was.is_none() || was == name_at(level, pos) {
        return;
    }
    let [x, y, z] = pos;
    let cells = (-1..=1).flat_map(|dx| (-1..=1).flat_map(move |dy| (-1..=1).map(move |dz| [x + dx, y + dy, z + dz])));
    let marked: Vec<(BlockPos, u32)> = cells
        .filter(|cell| level.in_height(cell[1]))
        .filter_map(|cell| {
            let id = level.block(cell[0], cell[1], cell[2]);
            let state = registry.get(id).filter(|state| is_leaves(state.name) && !is_set(state.properties, MARK) && !is_set(state.properties, KEPT))?;
            Some((cell, with_property(registry, id, MARK, "1").filter(|_| !state.is_air())?))
        })
        .collect();
    for (cell, id) in marked {
        send::set(world, layer, cell, id);
    }
}

/// Whether wood is within reach of the leaves at `pos`, going through leaves.
pub(super) fn hangs(level: &Level, pos: BlockPos) -> bool {
    let (mut seen, mut edge) = (HashSet::from([pos]), vec![pos]);
    for _ in 0..REACH {
        let mut next = Vec::new();
        for [x, y, z] in edge {
            for cell in AROUND.map(|[dx, dy, dz]| [x + dx, y + dy, z + dz]) {
                match name_at(level, cell) {
                    Some(name) if is_wood(name) => return true,
                    Some(name) if is_leaves(name) && seen.insert(cell) => next.push(cell),
                    _ => {}
                }
            }
        }
        edge = next;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_marked_leaves_that_are_not_a_players_wait_to_decay() {
        let waits = ["update_bit=1,persistent_bit=0", "update_bit=0,persistent_bit=0", "update_bit=1,persistent_bit=1"].map(|properties| is_marked("minecraft:oak_leaves", properties));
        assert_eq!(waits, [true, false, false]);
        assert!(!is_marked("minecraft:oak_log", "update_bit=1") && is_wood("minecraft:stripped_oak_log") && !is_wood("minecraft:oak_planks"));
    }
}
