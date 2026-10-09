//! Where v2 template placement turns a Java state into another Bedrock block than the legacy
//! loader the state table was probed through (docs/research/worldgen-bds-jigsaw.md, "V2 states";
//! read off the oracle's trial chambers).

use super::{Rotation, State, try_named};
use crate::block::Block;

const CARDINALS: [&str; 4] = ["north", "east", "south", "west"];
/// A trapdoor's `direction` (0 east, 1 west, 2 south, 3 north) turned a quarter clockwise.
const TRAPDOOR_CLOCKWISE: [usize; 4] = [2, 3, 1, 0];
/// The legacy loader's trapdoor `direction` -> the v2 one's (the Java facing's 2D index).
const TRAPDOOR_BASE: [usize; 4] = [3, 1, 0, 2];

fn value<'a>(states: &'a str, key: &str) -> Option<&'a str> {
    states.split(',').filter_map(|pair| pair.split_once('=')).find(|(k, _)| *k == key).map(|(_, v)| v)
}

fn with(block: Block, key: &str, new: &str) -> Block {
    let pairs = block.states().split(',').map(|pair| match pair.split_once('=') {
        Some((k, _)) if k == key => format!("{k}={new}"),
        _ => pair.to_string(),
    });
    try_named(block.name(), &pairs.collect::<Vec<_>>().join(","), block.is_waterlogged()).unwrap_or(block)
}

/// The block a v2 piece turned `rotation` places for `state`.
pub fn v2_block(state: State, rotation: Rotation) -> Block {
    let block = state.block(rotation);
    let (name, states) = (block.name(), block.states());
    let turns = rotation as usize;
    // Every template's spawner is Java's `waiting_for_players`, which the legacy loader drops.
    if name == "trial_spawner" {
        return with(block, "trial_spawner_state", "1");
    }
    if name == "decorated_pot" {
        let Some(d) = value(states, "direction").and_then(|d| d.parse::<usize>().ok()) else { return block };
        return with(block, "direction", &((d + 2) % 4).to_string());
    }
    if name.ends_with("trapdoor") {
        let unturned = state.block(Rotation::None);
        let Some(d) = value(unturned.states(), "direction").and_then(|d| d.parse::<usize>().ok()) else { return block };
        let turned = (0..turns).fold(TRAPDOOR_BASE[d], |d, _| TRAPDOOR_CLOCKWISE[d]);
        return with(block, "direction", &turned.to_string());
    }
    if name.ends_with("_door") {
        let key = "minecraft:cardinal_direction";
        let Some(at) = value(states, key).and_then(|c| CARDINALS.iter().position(|n| *n == c)) else { return block };
        // The lower half is the Java facing turned, a quarter short of the legacy block; the upper half turns from south.
        let turned = if value(states, "upper_block_bit") == Some("1") { (2 + turns) % 4 } else { (at + 3) % 4 };
        return with(block, key, CARDINALS[turned]);
    }
    block
}
