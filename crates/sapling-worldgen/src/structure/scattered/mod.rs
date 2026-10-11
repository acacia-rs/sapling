//! Desert pyramids, jungle temples and swamp huts (`RandomScatteredLargeFeature::
//! createStructureStart` 14835a7b0): one piece at (x*16, 64, z*16) facing south, picked by the
//! point biome at the start chunk's centre. Igloos (biomes 12, 30, 184) are templates, not here.
//! Each piece takes its height in the first chunk BDS post-processes it in, and keeps it.

mod cellar;
mod desert;
mod hut;
mod jungle;

use super::BoundingBox;
use super::canvas::Canvas;
use super::piece::{EAST, Frame, SOUTH, WEST};
use crate::block::Block;
use crate::random::Mt19937;
pub use desert::create_chest;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Temple {
    DesertPyramid,
    JungleTemple,
    SwampHut,
}

#[derive(Debug, Clone)]
pub struct Scattered {
    pub temple: Temple,
    /// The box before the height is known (y 64).
    pub bb: BoundingBox,
}

/// Legacy facing data for east, west, south and north, as BDS passes it to `getOrientationData`.
pub const STAIRS_DATA: [u8; 4] = [0, 1, 2, 3];

/// The stairs of `family` (by weirdo_direction) for facing `data` in the piece's orientation, by
/// `getOrientationData`'s table (141f4a1d0).
pub fn stairs(f: &Frame, family: [Block; 4], data: u8) -> Block {
    let table: [u8; 4] = match f.orientation {
        Some(SOUTH) => [0, 1, 3, 2],
        Some(WEST) => [2, 3, 0, 1],
        Some(EAST) => [2, 3, 1, 0],
        _ => [0, 1, 2, 3],
    };
    family[table[data as usize] as usize]
}

impl Scattered {
    pub fn new((x, z): (i32, i32), biome: u32) -> Option<Self> {
        let (temple, (w, h, d)) = match biome {
            2 | 17 => (Temple::DesertPyramid, (21, 15, 21)),
            21 | 22 => (Temple::JungleTemple, (12, 10, 15)),
            6 | 134 => (Temple::SwampHut, (7, 7, 9)),
            _ => return None,
        };
        let (x, z) = (x * 16, z * 16);
        Some(Self { temple, bb: BoundingBox::new(x, 64, z, x + w - 1, 64 + h - 1, z + d - 1) })
    }

    pub fn start(&self) -> (i32, i32) {
        (self.bb.x0 >> 4, self.bb.z0 >> 4)
    }

    /// The piece's blocks in the canvas's chunk, and its box there. `taken` gives the height a
    /// desert pyramid took in a chunk post-processed before this one; otherwise it takes it here, or
    /// with `start_random` (the start chunk's structure generator) as the start chunk would.
    pub fn post_process(&self, canvas: &mut Canvas, random: &mut Mt19937, taken: &dyn Fn((i32, i32)) -> Option<i32>, start_random: Option<Mt19937>) -> Option<BoundingBox> {
        let mut frame = Frame { bb: self.bb, orientation: Some(SOUTH), depth: 0, keep: |_| false };
        let here = (canvas.bb.x0 >> 4, canvas.bb.z0 >> 4);
        match self.temple {
            Temple::DesertPyramid => {
                let drawn = random.next_int(3);
                let (first, lowered) = match start_random {
                    Some(mut start) if self.start() != here => (self.start(), start.next_int(3)),
                    _ => (here, drawn),
                };
                let y = taken(self.start()).or_else(|| self.columns(first).map(|(x, z)| canvas.floor_height(x, z)).min().map(|lowest| lowest - lowered))?;
                frame.bb.move_y(y - frame.bb.y0);
                desert::post_process(&frame, canvas, random);
            }
            Temple::SwampHut => {
                frame.bb.move_y(self.average_ground(canvas, here)? + 1 - frame.bb.y0);
                hut::post_process(&frame, canvas, random);
            }
            Temple::JungleTemple => {
                frame.bb.move_y(self.average_ground(canvas, here)? - frame.bb.y0);
                jungle::post_process(&frame, canvas, random);
            }
        }
        Some(frame.bb)
    }

    /// The box's columns inside a chunk.
    fn columns(&self, (cx, cz): (i32, i32)) -> impl Iterator<Item = (i32, i32)> + use<> {
        let b = self.bb;
        let (x0, x1) = (b.x0.max(cx * 16), b.x1.min(cx * 16 + 15));
        let (z0, z1) = (b.z0.max(cz * 16), b.z1.min(cz * 16 + 15));
        (z0..=z1).flat_map(move |z| (x0..=x1).map(move |x| (x, z)))
    }

    /// `updateAverageGroundHeight` (148360c00): the mean top, water counted, over the box's
    /// columns in the chunk; None when it has none.
    fn average_ground(&self, canvas: &Canvas, chunk: (i32, i32)) -> Option<i32> {
        let (sum, count) = self.columns(chunk).fold((0, 0), |(sum, count), (x, z)| (sum + canvas.top(x, z, true), count + 1));
        (count > 0).then(|| sum / count)
    }
}
