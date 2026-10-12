//! `BlueIceFeature` (vtable slot 4, 14827ec60): under sea level, in or over water, beside packed ice
//! (not below); then 200 tries grow blue ice next to 1..4 blue ice blocks.

use super::SEA_LEVEL;
use crate::block::Block;
use crate::feature::view::View;
use crate::random::Mt19937;

const SIDES: [[i32; 3]; 6] = [[0, -1, 0], [0, 1, 0], [0, 0, -1], [0, 0, 1], [-1, 0, 0], [1, 0, 0]];

fn at(view: &View, [x, y, z]: [i32; 3], [dx, dy, dz]: [i32; 3]) -> Block {
    view.block(x + dx, y + dy, z + dz)
}

pub(super) fn blue_ice(view: &mut View, random: &mut Mt19937, origin: [i32; 3]) {
    if origin[1] >= SEA_LEVEL || (!at(view, origin, [0; 3]).is_water() && !at(view, origin, SIDES[0]).is_water()) {
        return;
    }
    if !SIDES[1..].iter().any(|&side| at(view, origin, side) == Block::PackedIce) {
        return;
    }
    view.set_block(origin[0], origin[1], origin[2], Block::BlueIce);
    for _ in 0..200 {
        let dy = random.next_int(5) - random.next_int(6);
        if dy <= -6 {
            continue;
        }
        let k = if dy < 2 { dy / 2 + 3 } else { 3 };
        let (a, b, c, d) = (random.next_int(k), random.next_int(k), random.next_int(k), random.next_int(k));
        let pos = [origin[0] + a - b, origin[1] + dy, origin[2] + c - d];
        let there = at(view, pos, [0; 3]);
        if !(matches!(there, Block::Air | Block::PackedIce | Block::Ice) || there.is_water()) {
            continue;
        }
        let blue = SIDES.iter().filter(|&&side| at(view, pos, side) == Block::BlueIce).count();
        if (1..=4).contains(&blue) {
            view.set_block(pos[0], pos[1], pos[2], Block::BlueIce);
        }
    }
}
