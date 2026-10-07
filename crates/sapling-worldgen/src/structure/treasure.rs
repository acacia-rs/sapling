//! Buried treasure (`BuriedTreasurePiece::postProcess` 148312c40): one chest at the start
//! chunk's (8, 8), 4 or 5 below the heightmap and lower until it sits in solid ground.

use super::canvas::Canvas;
use super::mineshaft::full_solid;
use crate::block::Block;
use crate::random::Mt19937;

const MIN_Y: i32 = 6;

pub fn place(canvas: &mut Canvas, random: &mut Mt19937, start: (i32, i32)) {
    let (x, z) = (start.0 * 16 + 8, start.1 * 16 + 8);
    if !canvas.bb.contains(x, 1, z) {
        return;
    }
    let solid = |canvas: &Canvas, x: i32, y: i32, z: i32| {
        let block = canvas.get(x, y, z);
        full_solid(block) && !block.is_fluid()
    };
    let mut y = canvas.floor_height(x, z) - random.next_int(2) - 4;
    // Down until the cell is enclosed above and on its four sides.
    loop {
        let enclosed = [(0, 1, 0), (0, 0, -1), (1, 0, 0), (0, 0, 1), (-1, 0, 0)].iter().all(|(dx, dy, dz)| solid(canvas, x + dx, y + dy, z + dz));
        let low = y < MIN_Y;
        y -= 1;
        if enclosed || low {
            break;
        }
    }
    let mut chest = y + 1;
    while !(solid(canvas, x, chest - 1, z) || chest < MIN_Y) {
        chest -= 1;
    }
    canvas.set(x, chest, z, Block::Chest);
}
