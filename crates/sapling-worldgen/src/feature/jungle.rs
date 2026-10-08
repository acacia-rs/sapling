//! The jungle features BDS builds in code (registered in 1482f4580): `BambooFeature::place`
//! (14827b960), `PodzolAreaFeature::place` (149d5fdf0) and `VinesFeature::place` (149d99a70).

use super::land::spread;
use super::lush::sturdy;
use super::tree::common::vine_facing;
use super::view::View;
use crate::block::Block;
use crate::random::Mt19937;
use crate::terrain::{HEIGHT, MIN_Y};

/// On air over grass, podzol, dirt or rooted dirt: `nextInt(8..=12)` + 5 thick stalks up while
/// the cells are air, the bound hashed from the column; three or more get leaves at the top.
pub fn bamboo(view: &mut View, random: &mut Mt19937, [x, y, z]: [i32; 3]) -> bool {
    if view.block(x, y, z) != Block::Air {
        return false;
    }
    if !matches!(view.block(x, y - 1, z), Block::GrassBlock | Block::Podzol | Block::Dirt | Block::DirtWithRoots) {
        return false;
    }
    let h = (x.wrapping_mul(0x2F_C20F) ^ z.wrapping_mul(0x6EB_FFF5)) as u32;
    let h = h.wrapping_mul(0x285_B825).wrapping_add(0xB).wrapping_mul(h);
    let bound = ((h >> 24) % 5) | 8;
    let count = (random.next_int(bound as i32) + 4).max(0) + 1;
    let mut top = y;
    for _ in 0..count {
        if view.block(x, top, z) != Block::Air {
            break;
        }
        view.set_block(x, top, z, Block::Bamboo);
        top += 1;
    }
    if top - y >= 3 {
        view.set_block(x, top, z, Block::BambooLargeLeaves);
        view.set_block(x, top - 1, z, Block::BambooLargeLeaves);
        view.set_block(x, top - 2, z, Block::BambooSmallLeaves);
    }
    true
}

/// Dirt and grass blocks one under the position within `nextInt(4) + 1` (a disc, x outer)
/// become podzol; true when any did.
pub fn podzol_area(view: &mut View, random: &mut Mt19937, [x, y, z]: [i32; 3]) -> bool {
    let r = random.next_int(4) + 1;
    let mut placed = false;
    for px in x - r..=x + r {
        for pz in z - r..=z + r {
            let (dx, dz) = (px - x, pz - z);
            if dx * dx + dz * dz <= r * r && matches!(view.block(px, y - 1, pz), Block::Dirt | Block::GrassBlock) {
                view.set_block(px, y - 1, pz, Block::Podzol);
                placed = true;
            }
        }
    }
    placed
}

/// The faces `VineBlock::mayPlace` is asked in (north, south, west, east): each is the face of
/// the supporting block, which sits on the opposite side of the vine.
const SUPPORTS: [(i32, i32); 4] = [(0, 1), (0, -1), (1, 0), (-1, 0)];

/// From y 64 up to the highest block of the chunks within 3 columns: on air, a vine on the first
/// supported side; on anything else the column moves to the position ±3 (two draws per axis).
pub fn vines(view: &mut View, random: &mut Mt19937, [x, _, z]: [i32; 3]) -> bool {
    let mut top = 0;
    for cx in x - 3..=x + 3 {
        for cz in z - 3..=z + 3 {
            top = view.highest_block(cx, cz).map_or(top, |h| top.max(h));
        }
    }
    if top == 0 {
        top = MIN_Y + HEIGHT as i32;
    }
    let (mut cx, mut cz) = (x, z);
    for y in 64..top {
        if view.block(cx, y, cz) == Block::Air {
            if let Some(&(dx, dz)) = SUPPORTS.iter().find(|&&(dx, dz)| sturdy(view.block(cx + dx, y, cz + dz))) {
                view.set_block(cx, y, cz, vine_facing((-dx, -dz)));
            }
        } else {
            cx = x + spread(random, 3);
            cz = z + spread(random, 3);
        }
    }
    true
}
