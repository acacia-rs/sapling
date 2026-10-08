//! The pale garden features BDS builds in code (registered in 1482f4580): `pale_moss_vegetation_feature`
//! (place 149d99710, weights set in 149e815b0) with `PaleMossCarpetBlock::placeAt` (1465f64a0), and
//! `eyeblossom_feature` (149e82050).

use super::cave::pick_weighted;
use super::plants::{is_soil, may_place};
use super::view::View;
use crate::block::Block;
use crate::random::Mt19937;

const VEGETATION: [(Block, f32); 3] = [(Block::PaleMossCarpet0, 25.0), (Block::ShortGrass, 25.0), (Block::TallGrass, 10.0)];

/// North, south, west, east: the order `placeAt` asks the sides in.
const SIDES: [(i32, i32); 4] = [(0, -1), (0, 1), (-1, 0), (1, 0)];

/// A neighbour face a carpet's side clings to.
fn supports(block: Block) -> bool {
    block.blocks_motion() && !block.is_leaves()
}

/// On air over a supporting block: a pale moss carpet, short grass or tall grass (upper half first).
pub fn vegetation(view: &mut View, random: &mut Mt19937, [x, y, z]: [i32; 3]) -> bool {
    let Some(block) = pick_weighted(&VEGETATION, random) else { return false };
    if view.block(x, y, z) != Block::Air || !view.block(x, y - 1, z).blocks_motion() {
        return false;
    }
    if block == Block::PaleMossCarpet0 {
        carpet(view, random, [x, y, z]);
        return true;
    }
    if !may_place(view, [x, y, z], block) {
        return false;
    }
    if block == Block::TallGrass {
        view.set_block(x, y + 1, z, Block::TallGrassTop);
    }
    view.set_block(x, y, z, block)
}

/// Sides cling to supporting neighbours; with air above, each side draws a float and over 0.5
/// grows tall (and gets an upper half) where the neighbour above supports it too.
fn carpet(view: &mut View, random: &mut Mt19937, [x, y, z]: [i32; 3]) {
    let open_above = view.block(x, y + 1, z) == Block::Air;
    let mut base = [0usize; 4];
    let mut top = [0usize; 4];
    for (i, &(dx, dz)) in SIDES.iter().enumerate() {
        let wall = supports(view.block(x + dx, y, z + dz));
        if !open_above {
            base[i] = usize::from(wall);
            continue;
        }
        let tall = random.next_float() > 0.5 && supports(view.block(x + dx, y + 1, z + dz));
        base[i] = if wall { 1 + usize::from(tall) } else { 0 };
        top[i] = usize::from(wall && tall);
    }
    let [n, s, w, e] = base;
    view.set_block(x, y, z, Block::ALL[Block::PaleMossCarpet0.index() + n * 27 + e * 9 + s * 3 + w]);
    if top.iter().any(|&t| t != 0) {
        let [n, s, w, e] = top;
        view.set_block(x, y + 1, z, Block::ALL[Block::PaleMossCarpetTop0.index() + n * 8 + e * 4 + s * 2 + w]);
    }
}

/// A closed eyeblossom (generation runs at day) on air over soil.
pub fn eyeblossom(view: &mut View, [x, y, z]: [i32; 3]) -> bool {
    let placed = view.block(x, y, z) == Block::Air && is_soil(view.block(x, y - 1, z));
    placed && view.set_block(x, y, z, Block::ClosedEyeblossom)
}
