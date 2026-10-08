//! Stair corners: Bedrock 1.26 saves `minecraft:corner` from the stairs' final neighbours, by
//! Java's `StairBlock.getStairsShape` (weirdo_direction 0..3 = east, west, south, north).

use super::canvas::Canvas;
use crate::block::Block;

/// (dx, dz) per weirdo_direction.
const STEP: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
/// The direction counter-clockwise from each (east -> north, west -> south, south -> east, north -> west).
const COUNTER_CLOCKWISE: [u8; 4] = [3, 2, 0, 1];

struct Stairs {
    weirdo: u8,
    upside_down: u8,
}

fn parse(block: Block) -> Option<Stairs> {
    if !block.name().ends_with("_stairs") {
        return None;
    }
    let value = |key: &str| block.states().split(',').find_map(|kv| kv.strip_prefix(key)?.strip_prefix('='));
    Some(Stairs { weirdo: value("weirdo_direction")?.parse().ok()?, upside_down: value("upside_down_bit")?.parse().ok()? })
}

pub fn is_stairs(block: Block) -> bool {
    parse(block).is_some()
}

/// The stairs at (x, y, z) with the corner its neighbours give; the block unchanged when the
/// table lacks that variant.
pub fn rejoin(canvas: &Canvas, (x, y, z): (i32, i32, i32), block: Block) -> Block {
    let Some(me) = parse(block) else { return block };
    let at = |d: u8, sign: i32| {
        let (dx, dz) = STEP[d as usize];
        canvas.get(x + dx * sign, y, z + dz * sign)
    };
    let neighbour = |d: u8, sign: i32| parse(at(d, sign)).filter(|s| s.upside_down == me.upside_down);
    let axis = |d: u8| d / 2;
    // Java's canTakeShape: the side cell is not stairs of the same facing and half.
    let free = |d: u8, sign: i32| neighbour(d, sign).is_none_or(|s| s.weirdo != me.weirdo);
    let ccw = COUNTER_CLOCKWISE[me.weirdo as usize];
    let opposite = |d: u8| d ^ 1;
    let corner = if let Some(back) = neighbour(me.weirdo, 1).filter(|b| axis(b.weirdo) != axis(me.weirdo) && free(opposite(b.weirdo), 1)) {
        if back.weirdo == ccw { "outer_left" } else { "outer_right" }
    } else if let Some(front) = neighbour(me.weirdo, -1).filter(|f| axis(f.weirdo) != axis(me.weirdo) && free(f.weirdo, 1)) {
        if front.weirdo == ccw { "inner_left" } else { "inner_right" }
    } else {
        "none"
    };
    let states = format!("minecraft:corner={corner},upside_down_bit={},weirdo_direction={}", me.upside_down, me.weirdo);
    Block::with_states(block.name(), &states).unwrap_or(block)
}
