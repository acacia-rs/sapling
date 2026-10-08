//! The stronghold portal room (`StrongholdPortalRoom::postProcess` 1480a1b20).

use super::Door;
use super::common::{fill, portal_frame, selector_box, small_door};
use crate::block::Block;
use crate::random::Mt19937;
use crate::structure::canvas::Canvas;
use crate::structure::piece::Frame;
use crate::structure::scattered::stairs;

pub fn portal_room(f: &Frame, canvas: &mut Canvas, random: &mut Mt19937) {
    selector_box(f, canvas, random, (0, 0, 0), (10, 7, 15));
    small_door(f, canvas, Door::Grates, (4, 1, 0));
    selector_box(f, canvas, random, (1, 6, 1), (1, 6, 14));
    selector_box(f, canvas, random, (9, 6, 1), (9, 6, 14));
    selector_box(f, canvas, random, (2, 6, 1), (8, 6, 2));
    selector_box(f, canvas, random, (2, 6, 14), (8, 6, 14));
    selector_box(f, canvas, random, (1, 1, 1), (2, 1, 4));
    selector_box(f, canvas, random, (8, 1, 1), (9, 1, 4));
    fill(f, canvas, (1, 1, 1), (1, 1, 3), Block::Lava);
    fill(f, canvas, (9, 1, 1), (9, 1, 3), Block::Lava);
    selector_box(f, canvas, random, (3, 1, 8), (7, 1, 12));
    fill(f, canvas, (4, 1, 9), (6, 1, 11), Block::Lava);
    for z in (3..14).step_by(2) {
        fill(f, canvas, (0, 3, z), (0, 4, z), Block::IronBars0);
        fill(f, canvas, (10, 3, z), (10, 4, z), Block::IronBars0);
    }
    for x in (2..9).step_by(2) {
        fill(f, canvas, (x, 3, 15), (x, 4, 15), Block::IronBars0);
    }
    let step = stairs(f, [Block::StoneBrickStairsEast, Block::StoneBrickStairsWest, Block::StoneBrickStairsSouth, Block::StoneBrickStairsNorth], 3);
    selector_box(f, canvas, random, (4, 1, 5), (6, 1, 7));
    selector_box(f, canvas, random, (4, 2, 6), (6, 2, 7));
    selector_box(f, canvas, random, (4, 3, 7), (6, 3, 7));
    for x in 4..=6 {
        f.place(canvas, step, x, 1, 4);
        f.place(canvas, step, x, 2, 5);
        f.place(canvas, step, x, 3, 6);
    }
    // Legacy direction of each side's frames: the near row, the far row, the x 3 and x 7 columns.
    let o = f.orientation.unwrap_or(0) as usize;
    let facing = [o as u8, [2, 3, 0, 1][o], [3, 0, 3, 0][o], [1, 2, 1, 2][o]];
    let eyes: [bool; 12] = std::array::from_fn(|_| random.next_float() > 0.9);
    let cells = [(4, 8), (5, 8), (6, 8), (4, 12), (5, 12), (6, 12), (3, 9), (3, 10), (3, 11), (7, 9), (7, 10), (7, 11)];
    for (i, &(x, z)) in cells.iter().enumerate() {
        f.place(canvas, portal_frame(facing[i / 3], eyes[i]), x, 3, z);
    }
    if eyes.iter().all(|&e| e) {
        for z in 9..=11 {
            (4..=6).for_each(|x| f.place(canvas, Block::EndPortal, x, 3, z));
        }
    }
    f.place(canvas, Block::MobSpawner, 5, 3, 6);
}
