//! Warm ocean coral, BDS's own (no Java counterpart): `CoralCrustFeature` (crust.rs), `CoralFeature`
//! (shape.rs builds, this file decorates), `CoralHangFeature` and `SeaAnemoneFeature`. Each rule has its
//! own generator, so the later ones depend on the earlier only through the blocks they left.
//! Decompile map: docs/research/worldgen-bds-caves.md, "Coral".

mod arm;
mod crust;
mod shape;

pub use crust::{crust, set_dead_coral};

use super::aquatic;
use super::view::View;
use crate::block::Block;
use crate::density::SEA_LEVEL;
use crate::random::Mt19937;

/// `Facing` offsets (0x14a712760): down, up, north, south, west, east.
const OFFSETS: [[i32; 3]; 6] = [[0, -1, 0], [0, 1, 0], [0, 0, -1], [0, 0, 1], [-1, 0, 0], [1, 0, 0]];
/// The opposite face (0x14a682118).
const OPPOSITE: [usize; 6] = [1, 0, 3, 2, 5, 4];
/// `coral_direction` of a wall fan on the face north, south, west, east (0x14a7198f8).
const WALL_DIRECTION: [usize; 4] = [2, 3, 0, 1];
/// The faces a side decoration may take, by the face it was recorded with (0x14a925e68).
const SIDE_FACES: [[usize; 3]; 4] = [[5, 3, 4], [5, 2, 4], [5, 3, 2], [2, 3, 4]];

/// By `CoralColor` (tube, brain, bubble, fire, horn), live then dead (list 0x14ccda7e8).
const CORAL_BLOCKS: [Block; 10] = [
    Block::TubeCoralBlock,
    Block::BrainCoralBlock,
    Block::BubbleCoralBlock,
    Block::FireCoralBlock,
    Block::HornCoralBlock,
    Block::DeadTubeCoralBlock,
    Block::DeadBrainCoralBlock,
    Block::DeadBubbleCoralBlock,
    Block::DeadFireCoralBlock,
    Block::DeadHornCoralBlock,
];
const PLANTS: [Block; 5] = [Block::TubeCoral, Block::BrainCoral, Block::BubbleCoral, Block::FireCoral, Block::HornCoral];

fn fan(color: usize, direction: usize) -> Block {
    Block::ALL[Block::TubeCoralFan0.index() + color * 2 + direction]
}

fn wall_fan(color: usize, direction: usize) -> Block {
    Block::ALL[Block::TubeCoralWallFan0.index() + color * 4 + direction]
}

fn step([x, y, z]: [i32; 3], face: usize) -> [i32; 3] {
    let [dx, dy, dz] = OFFSETS[face];
    [x + dx, y + dy, z + dz]
}

fn up([x, y, z]: [i32; 3], n: i32) -> [i32; 3] {
    [x, y + n, z]
}

fn coral_index(block: Block) -> Option<usize> {
    CORAL_BLOCKS.iter().position(|&b| b == block)
}

/// The block's material is water (fluid water, not a waterlogged block).
fn is_water(block: Block) -> bool {
    block.is_fluid() && !block.is_lava()
}

fn block(view: &View, [x, y, z]: [i32; 3]) -> Block {
    view.block(x, y, z)
}

fn solid(view: &View, [x, y, z]: [i32; 3]) -> bool {
    aquatic::solid(view, x, y, z)
}

/// `getLiquidBlock` is water there: a water block or a waterlogged one.
fn submerged(view: &View, p: [i32; 3]) -> bool {
    let b = block(view, p);
    is_water(b) || b.is_waterlogged()
}

/// `BlockSource::isUnderWater` (1416027c0) for a solid coral block: water above, nothing of air
/// material below or beside.
fn fits(view: &View, p: [i32; 3]) -> bool {
    submerged(view, up(p, 1)) && [0, 2, 3, 4, 5].iter().all(|&f| block(view, step(p, f)) != Block::Air)
}

/// `CoralFeature::_canOverwrite`: anything but a coral block of another colour (dead ones included).
fn can_overwrite(view: &View, p: [i32; 3], color: usize) -> bool {
    coral_index(block(view, p)).is_none_or(|i| i == color)
}

/// What the coral builders share: the world, the placing generator and the decoration lists
/// (`topDec`, `sideDec` with a face).
pub struct Reef<'r, 'v> {
    view: &'r mut View<'v>,
    random: &'r mut Mt19937,
    tops: Vec<[i32; 3]>,
    sides: Vec<([i32; 3], usize)>,
}

impl<'r, 'v> Reef<'r, 'v> {
    fn new(view: &'r mut View<'v>, random: &'r mut Mt19937) -> Self {
        Self { view, random, tops: Vec::new(), sides: Vec::new() }
    }

    fn set(&mut self, [x, y, z]: [i32; 3], b: Block) -> bool {
        self.view.set_block(x, y, z, b)
    }

    /// `_canOverwrite`, then the coral block.
    fn try_set(&mut self, p: [i32; 3], color: usize) -> bool {
        let ok = can_overwrite(self.view, p, color);
        if ok {
            self.set(p, CORAL_BLOCKS[color]);
        }
        ok
    }

    /// A fan on each top that stands in water on a coral, then a wall fan beside each side.
    fn decorate(&mut self) {
        for t in std::mem::take(&mut self.tops) {
            let view = &*self.view;
            if !is_water(block(view, t)) || is_water(block(view, up(t, -1))) || !solid(view, up(t, -1)) {
                continue;
            }
            let color = self.random.next_int(5) as usize;
            let direction = self.random.next_int(2) as usize;
            self.set(t, fan(color, direction));
        }
        for (p, recorded) in std::mem::take(&mut self.sides) {
            let at = block(self.view, p);
            let on_coral = coral_index(at).is_some() || at == Block::Stone;
            let face = SIDE_FACES.get(recorded.wrapping_sub(2)).unwrap_or(&SIDE_FACES[2])[self.random.next_int(3) as usize];
            let q = step(p, face);
            if !is_water(block(self.view, q)) || !on_coral || !submerged(self.view, q) {
                continue;
            }
            let color = (self.random.next_int(2) + 2 * self.random.next_int(3)).clamp(0, 4) as usize;
            self.set(q, wall_fan(color, WALL_DIRECTION[face - 2]));
        }
    }
}

/// `CoralFeature::place` (14828af70): up to the first water below sea level - 4, on a solid floor
/// that is not live coral, a coral of a random colour, then its decorations.
pub fn coral(view: &mut View, random: &mut Mt19937, [x, mut y, z]: [i32; 3]) -> bool {
    while y < SEA_LEVEL - 4 && !is_water(view.block(x, y, z)) {
        y += 1;
    }
    let p = [x, y, z];
    if y >= SEA_LEVEL - 4 || !solid(view, up(p, -1)) || coral_index(block(view, up(p, -1))).is_some_and(|i| i < 5) {
        return false;
    }
    let color = random.next_int(5) as usize;
    let mut reef = Reef::new(view, random);
    reef.place_coral(p, color);
    reef.decorate();
    true
}

/// `CoralHangFeature::place` (148294070): down to a coral or stone block, a side of it in water
/// (E S W N, each failed pick swapped out of the draw), and a tube or brain wall fan there.
pub fn hang(view: &mut View, random: &mut Mt19937, at: [i32; 3]) -> bool {
    let mut p = at;
    while !solid(view, p) && p[1] > SEA_LEVEL - 13 {
        p[1] -= 1;
    }
    let on = block(view, p);
    if !solid(view, p) || (coral_index(on).is_none() && on != Block::Stone) {
        return false;
    }
    let mut faces = [5, 3, 4, 2];
    let mut chosen = 2;
    for n in (1..=4).rev() {
        let k = random.next_int(n as i32) as usize;
        if is_water(block(view, step(p, faces[k]))) {
            chosen = faces[k];
            break;
        }
        faces.swap(k, n - 1);
    }
    // Gotcha: BDS tests the water beside the rule's position, not beside the block it found.
    if !is_water(block(view, step(p, chosen))) || !submerged(view, step(at, chosen)) {
        return false;
    }
    let color = if random.next_int(2) > 0 { 1 } else { 0 };
    let [x, y, z] = step(p, chosen);
    view.set_block(x, y, z, wall_fan(color, WALL_DIRECTION[chosen - 2]));
    true
}

/// `SeaAnemoneFeature::place` (149d60d10): down to a solid floor above sea level - 15, then in
/// water a fan (1 in 4) or a coral plant of a random colour.
pub fn anemone(view: &mut View, random: &mut Mt19937, [x, mut y, z]: [i32; 3]) -> bool {
    let limit = SEA_LEVEL - 15;
    while !solid(view, [x, y - 1, z]) && y > limit {
        y -= 1;
    }
    let p = [x, y, z];
    if !is_water(block(view, p)) || !solid(view, up(p, -1)) || y <= limit || !submerged(view, p) {
        return false;
    }
    let plant = if random.next_int(4) == 0 {
        let color = random.next_int(5) as usize;
        fan(color, random.next_int(2) as usize)
    } else {
        PLANTS[random.next_int(5) as usize]
    };
    view.set_block(x, y, z, plant);
    true
}
