//! Mirrored placement (ruined portals): BDS flips template z before turning it, so a placed block
//! is its turned state flipped across the world axis the template's z lands on. Neighbour-shaped
//! states (stair corners, connections) are set afterwards by `connect`.

use super::{Rotation, named};
use crate::block::Block;

/// Swaps of a horizontal state when flipping north and south (`z`) or east and west.
const SWAPS: [(&str, [(&str, &str); 2], [(&str, &str); 2]); 5] = [
    ("minecraft:cardinal_direction", [("north", "south"), ("south", "north")], [("east", "west"), ("west", "east")]),
    ("weirdo_direction", [("2", "3"), ("3", "2")], [("0", "1"), ("1", "0")]),
    ("facing_direction", [("2", "3"), ("3", "2")], [("4", "5"), ("5", "4")]),
    ("direction", [("0", "2"), ("2", "0")], [("1", "3"), ("3", "1")]),
    ("torch_facing_direction", [("north", "south"), ("south", "north")], [("east", "west"), ("west", "east")]),
];

/// `block`, already turned by `rotation`, flipped as a z-mirrored template turned that way is.
pub fn mirror(block: Block, rotation: Rotation) -> Block {
    let along_z = matches!(rotation, Rotation::None | Rotation::Clockwise180);
    let states = block.states();
    if states.is_empty() {
        return block;
    }
    let flipped: Vec<String> = states
        .split(',')
        .map(|pair| {
            let Some((key, value)) = pair.split_once('=') else { return pair.to_string() };
            let swaps = SWAPS.iter().find(|(k, _, _)| *k == key).map(|(_, z, x)| if along_z { z } else { x });
            let value = swaps.and_then(|s| s.iter().find(|(from, _)| *from == value)).map_or(value, |(_, to)| to);
            format!("{key}={value}")
        })
        .collect();
    let flipped = flipped.join(",");
    if flipped == states { block } else { named(block.name(), &flipped, block.is_waterlogged()) }
}
