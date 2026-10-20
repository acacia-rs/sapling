//! `WeepingVinesClusterFeature::place` (14829d8a0), `TwistingVinesClusterFeature::place` (149d66ee0) and the
//! weeping vine column they share with the huge fungus (1425e9460). Every vine cell is the vine block with its
//! own age: BDS has no separate plant block for the stem.

use super::view::View;
use crate::block::Block;
use crate::random::Mt19937;

const FACES: [[i32; 3]; 6] = [[0, -1, 0], [0, 1, 0], [0, 0, -1], [0, 0, 1], [-1, 0, 0], [1, 0, 0]];

/// `(nextInt(bound) + add) << (nextInt(6) == 0)`, or 1 when `nextInt(5) == 0`.
pub fn vine_length(random: &mut Mt19937, bound: i32, add: i32) -> i32 {
    let length = (random.next_int(bound) + add) << u32::from(random.next_int(6) == 0);
    if random.next_int(5) == 0 { 1 } else { length }
}

/// Down from `at` over `length + 1` cells, a vine with an age from `ages` (end exclusive) in each air cell,
/// stopping over a cell that is not air.
pub fn weeping_column(view: &mut View, random: &mut Mt19937, [x, mut y, z]: [i32; 3], length: i32, (min, max): (i32, i32)) {
    for i in 0..=length {
        if view.block(x, y, z) == Block::Air {
            let age = if min < max { random.next_int(max - min) + min } else { min };
            view.set_block(x, y, z, Block::weeping_vines(age));
            if i == length || view.block(x, y - 1, z) != Block::Air {
                break;
            }
        }
        y -= 1;
    }
}

fn holds_weeping(block: Block) -> bool {
    matches!(block, Block::Netherrack | Block::NetherWartBlock)
}

pub fn weeping_cluster(view: &mut View, random: &mut Mt19937, [x, y, z]: [i32; 3]) -> bool {
    if view.block(x, y, z) != Block::Air || !holds_weeping(view.block(x, y + 1, z)) {
        return false;
    }
    view.set_block(x, y, z, Block::NetherWartBlock);
    for _ in 0..200 {
        let [px, py, pz] = [x + random.next_int(12) - 6, y + random.next_int(7) - 5, z + random.next_int(12) - 6];
        let touching = FACES.iter().filter(|[dx, dy, dz]| holds_weeping(view.block(px + dx, py + dy, pz + dz))).count();
        if view.block(px, py, pz) == Block::Air && touching == 1 {
            view.set_block(px, py, pz, Block::NetherWartBlock);
        }
    }
    for _ in 0..100 {
        let [px, py, pz] = [x + random.next_int(16) - 8, y + random.next_int(9) - 8, z + random.next_int(16) - 8];
        if view.block(px, py, pz) == Block::Air && holds_weeping(view.block(px, py + 1, pz)) {
            let length = vine_length(random, 7, 1);
            weeping_column(view, random, [px, py, pz], length, (17, 25));
        }
    }
    true
}

/// No gate at the origin; each try is centred 5 blocks to the -x and -z of it.
pub fn twisting_cluster(view: &mut View, random: &mut Mt19937, [x, y, z]: [i32; 3]) -> bool {
    for _ in 0..100 {
        let px = x + random.next_int(10) - random.next_int(10) - 5;
        let pz = z + random.next_int(10) - random.next_int(10) - 5;
        let py = y + random.next_int(5) - 4;
        if view.block(px, py, pz) != Block::Air || !matches!(view.block(px, py - 1, pz), Block::Netherrack | Block::WarpedNylium | Block::WarpedWartBlock) {
            continue;
        }
        let length = vine_length(random, 7, 1);
        let mut cy = py;
        for i in 0..=length {
            if view.block(px, cy, pz) == Block::Air {
                view.set_block(px, cy, pz, Block::twisting_vines(random.next_int(25)));
                if i == length || view.block(px, cy + 1, pz) != Block::Air {
                    break;
                }
            }
            cy += 1;
        }
    }
    true
}
