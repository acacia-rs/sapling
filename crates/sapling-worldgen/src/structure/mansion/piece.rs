//! A mansion in one chunk (start postProcess 1480ab380): each piece meeting the chunk as a
//! `TemplateStructurePiece` (141f797b0, markers 1480b2360), then cobblestone under the start's
//! bottom layer wherever a piece stands, then the entrance's stone stairs (1480abb80), both
//! stepping down through air and fluids no lower than y 2 (BDS's legacy floor).

use super::{Mansion, Mirror, Piece, Pos};
use crate::block::Block;
use crate::structure::BoundingBox;
use crate::structure::canvas::Canvas;
use crate::structure::template::{Placement, Rotation, named};

/// A template offset as the piece's mirror and rotation place it (FRONT_BACK is LEFT_RIGHT
/// half a turn further).
pub fn transform(p: Pos, rotation: Rotation, mirror: Mirror) -> Pos {
    let (rotation, flip) = settings(rotation, mirror);
    rotation.apply(if flip { (p.0, p.1, -p.2) } else { p })
}

fn settings(rotation: Rotation, mirror: Mirror) -> (Rotation, bool) {
    match mirror {
        Mirror::None => (rotation, false),
        Mirror::LeftRight => (rotation, true),
        Mirror::FrontBack => (Rotation::from_index(rotation as i32 + 2), true),
    }
}

fn open(block: Block) -> bool {
    block == Block::Air || block.is_fluid()
}

/// From `y` down to y 2, cobblestone through air and fluids.
fn fill_down(canvas: &mut Canvas, (x, mut y, z): Pos) {
    let cobblestone = named("cobblestone", "", false);
    while y >= 2 && open(canvas.get(x, y, z)) {
        canvas.set(x, y, z, cobblestone);
        y -= 1;
    }
}

impl Piece {
    fn place(&self, canvas: &mut Canvas) {
        let (rotation, flip) = settings(self.rotation, self.mirror);
        let mut placement = Placement::new(rotation, canvas.bb);
        placement.mirror = flip;
        placement.place(canvas, self.template, self.origin);
        for (index, name) in &self.template.markers {
            let (dx, dy, dz) = transform(self.template.blocks[*index].pos, self.rotation, self.mirror);
            let (x, y, z) = (self.origin.0 + dx, self.origin.1 + dy, self.origin.2 + dz);
            if !canvas.bb.contains(x, y, z) {
                continue;
            }
            if name == "Group of Allays" {
                canvas.set(x, y, z, Block::Air);
            } else if let Some(side) = ["ChestNorth", "ChestEast", "ChestSouth", "ChestWest"].iter().position(|n| n == name) {
                let side = match (self.mirror, side) {
                    (Mirror::LeftRight, 0 | 2) | (Mirror::FrontBack, 1 | 3) => side + 2,
                    _ => side,
                };
                let facing = [Block::ChestNorth, Block::ChestEast, Block::ChestSouth, Block::ChestWest][(side + self.rotation as usize) & 3];
                if canvas.get(x, y, z).name() != "chest" {
                    canvas.set(x, y, z, facing);
                }
            }
        }
    }
}

impl Mansion {
    pub fn post_process(&self, canvas: &mut Canvas) {
        let column = canvas.bb;
        for piece in self.pieces.iter().filter(|p| p.bb.intersects(&column)) {
            piece.place(canvas);
        }
        let y0 = self.bb.y0;
        for x in column.x0..=column.x1 {
            for z in column.z0..=column.z1 {
                let inside = |b: &BoundingBox| b.contains(x, y0, z);
                if canvas.get(x, y0, z) != Block::Air && inside(&self.bb) && self.pieces.iter().any(|p| inside(&p.bb)) && y0 > 2 {
                    fill_down(canvas, (x, y0 - 1, z));
                }
            }
        }
        self.entrance_stairs(canvas);
    }

    /// Two pairs of stair runs down from the entrance, each stepping out and down while air.
    fn entrance_stairs(&self, canvas: &mut Canvas) {
        let (x, y, z) = self.origin;
        // (step, first run's facing, second run's facing, first stairs' offsets and weirdo direction, second's offsets)
        let (step, d1, d2, first, wd1, second) = match self.rotation {
            Rotation::None => ((1, 0), 2, 3, (9, 2), 2, (9, 12)),
            Rotation::Clockwise90 => ((0, 1), 5, 4, (-2, 9), 1, (-12, 9)),
            Rotation::Clockwise180 => ((-1, 0), 3, 2, (-9, -2), 3, (-9, -12)),
            Rotation::CounterClockwise90 => ((0, -1), 4, 5, (2, -9), 0, (12, -9)),
        };
        let wd2 = if (2..=4).contains(&d1) { 3 - (d1 - 2) } else { 0 };
        for (offset, wd, facing) in [(first, wd1, d1), (second, wd2, d2)] {
            let stairs = named("stone_stairs", &format!("minecraft:corner=none,upside_down_bit=0,weirdo_direction={wd}"), false);
            for k in 0..2 {
                let at = (x + offset.0 + step.0 * k, y - 1, z + offset.1 + step.1 * k);
                stair_run(canvas, at, stairs, facing);
            }
        }
    }
}

/// 1480abb80: from an air cell, stairs over cobblestone, one step out (by the facing) and down
/// at a time while the next cell is air, within the chunk.
fn stair_run(canvas: &mut Canvas, (mut x, mut y, mut z): Pos, stairs: Block, facing: i32) {
    let (sx, sz) = match facing {
        2 => (0, -1),
        3 => (0, 1),
        4 => (-1, 0),
        _ => (1, 0),
    };
    if canvas.get(x, y, z) != Block::Air {
        return;
    }
    loop {
        if !canvas.bb.contains(x, y, z) {
            break;
        }
        canvas.set(x, y, z, stairs);
        if y > 2 {
            fill_down(canvas, (x, y - 1, z));
        }
        (x, y, z) = (x + sx, y - 1, z + sz);
        if canvas.get(x, y, z) != Block::Air {
            break;
        }
    }
}
