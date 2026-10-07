//! The legacy lava lake of `decorationPostProcessChunk` (1482143a0): its trigger, and
//! `LakeFeature::place` (149e923b0), Java's pre-1.18 lake in f32.

use crate::block::Block;
use crate::random::Mt19937;
use crate::terrain::{CellKind, MIN_Y};

/// The dimension's `+0xcc`, compared with the lake's start height.
const SEA_LEVEL: i32 = 63;

/// 16x16x8 cells, `(x * 16 + z) * 8 + y`, from the lake's corner.
pub struct Lake {
    corner: [i32; 3],
    cells: Box<[bool; 2048]>,
}

fn index(x: i32, z: i32, y: i32) -> usize {
    ((x * 16 + z) * 8 + y) as usize
}

/// The lava lake attempt of a chunk; `random` is the chunk's decoration generator. `kind` reads
/// the carved terrain around it, across chunk borders.
pub fn lava_lake(random: &mut Mt19937, chunk_x: i32, chunk_z: i32, kind: &mut impl FnMut(i32, i32, i32) -> CellKind) -> Option<Lake> {
    if random.next_int(8) != 0 {
        return None;
    }
    let x = random.next_int(9);
    let bound = random.next_int(120) + 8;
    let y = random.next_int(bound);
    let z = random.next_int(9);
    if y < SEA_LEVEL - 3 || (y > SEA_LEVEL && random.next_int(10) != 0) {
        return None;
    }
    place(random, [chunk_x * 16 + 3 + x, y, chunk_z * 16 + 3 + z], kind)
}

fn place(random: &mut Mt19937, [x, mut y, z]: [i32; 3], kind: &mut impl FnMut(i32, i32, i32) -> CellKind) -> Option<Lake> {
    if MIN_Y + 4 >= y {
        return None;
    }
    let (x0, z0) = (x - 8, z - 8);
    while MIN_Y < y {
        if kind(x0, y, z0) != CellKind::Air || y < 1 {
            break;
        }
        y -= 1;
    }
    let corner = [x0, y - 4, z0];
    let mut cells = Box::new([false; 2048]);
    let blobs = random.next_int(4) + 4;
    for _ in 0..blobs {
        blob(random, &mut cells);
    }
    let lake = Lake { corner, cells };
    lake.fits(kind).then_some(lake)
}

/// One ellipsoid; the in-blob sums keep BDS's addition order.
fn blob(random: &mut Mt19937, cells: &mut [bool; 2048]) {
    let size_x = random.next_float() * 6.0 + 3.0;
    let size_y = random.next_float() * 4.0 + 2.0;
    let size_z = random.next_float() * 6.0 + 3.0;
    let center_x = ((16.0 - size_x) + -2.0) * random.next_float() + 1.0 + size_x * 0.5;
    let center_y = ((8.0 - size_y) + -4.0) * random.next_float() + 2.0 + size_y * 0.5;
    let center_z = ((16.0 - size_z) + -2.0) * random.next_float() + 1.0 + size_z * 0.5;
    let (half_x, half_y, half_z) = (size_x * 0.5, size_y * 0.5, size_z * 0.5);
    let dys: [f32; 6] = std::array::from_fn(|i| {
        let d = ((i + 1) as f32 - center_y) / half_y;
        d * d
    });
    for cx in 1..15 {
        let dx = (cx as f32 - center_x) / half_x;
        let dx2 = dx * dx;
        for cz in 1..15 {
            let dz = (cz as f32 - center_z) / half_z;
            let dz2 = dz * dz;
            for cy in 1..7 {
                let dy2 = dys[cy as usize - 1];
                let sum = if cy < 6 { (dy2 + dx2) + dz2 } else { dz2 + (dx2 + dy2) };
                if 1.0 > sum {
                    cells[index(cx, cz, cy)] = true;
                }
            }
        }
    }
}

impl Lake {
    fn filled(&self, x: i32, z: i32, y: i32) -> bool {
        (0..16).contains(&x) && (0..16).contains(&z) && (0..8).contains(&y) && self.cells[index(x, z, y)]
    }

    /// Every empty cell touching the lake: no liquid beside the air part, and solid or lava
    /// beside the lava part.
    fn fits(&self, kind: &mut impl FnMut(i32, i32, i32) -> CellKind) -> bool {
        for x in 0..16 {
            for z in 0..16 {
                for y in 0..8 {
                    let border = !self.filled(x, z, y)
                        && [(1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)]
                            .iter()
                            .any(|&(dx, dz, dy)| self.filled(x + dx, z + dz, y + dy));
                    if !border {
                        continue;
                    }
                    let cell = kind(self.corner[0] + x, self.corner[1] + y, self.corner[2] + z);
                    let liquid = matches!(cell, CellKind::Water | CellKind::Lava);
                    if (y >= 4 && liquid) || (y < 4 && cell != CellKind::Solid && cell != CellKind::Lava) {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// The `nextInt(2)` draws of turning the border into stone: one per border cell in the upper half.
    pub fn border_draws(&self) -> usize {
        let mut count = 0;
        for x in 0..16 {
            for z in 0..16 {
                for y in 4..8 {
                    let near = [(1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)];
                    if !self.filled(x, z, y) && near.iter().any(|&(dx, dz, dy)| self.filled(x + dx, z + dz, y + dy)) {
                        count += 1;
                    }
                }
            }
        }
        count
    }

    /// Writes the lake's cells inside one chunk: lava in its lower half, air above.
    // TODO: BDS then turns the lake's solid border into stone (and draws a nextInt(2) per border cell); shape is unchanged.
    pub fn apply(&self, chunk_x: i32, chunk_z: i32, mut set: impl FnMut(usize, i32, usize, Block)) {
        for x in 0..16 {
            for z in 0..16 {
                let (wx, wz) = (self.corner[0] + x, self.corner[2] + z);
                if wx.div_euclid(16) != chunk_x || wz.div_euclid(16) != chunk_z {
                    continue;
                }
                for y in 0..8 {
                    if self.filled(x, z, y) {
                        let block = if y < 4 { Block::Lava } else { Block::Air };
                        set(wx.rem_euclid(16) as usize, self.corner[1] + y, wz.rem_euclid(16) as usize, block);
                    }
                }
            }
        }
    }
}
