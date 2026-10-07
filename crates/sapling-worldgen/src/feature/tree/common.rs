//! What every trunk and canopy shares: BDS's int-range and chance draws, the cell tests and the
//! attachable decorations. Port of stirante/featurelab `features/tree.go` (MIT, (c) 2026 stirante).

use super::spec::{CanopyDecoration, Odds, Decoration, Decorative, Range, Step, Tree};
use crate::block::Block;
use crate::feature::view::View;
use crate::random::Mt19937;

impl Range {
    /// `IntRange::getValue`: `min + nextInt(max - min)`, no draw when `min >= max - 1`.
    pub fn draw(self, random: &mut Mt19937) -> i32 {
        if self.0 >= self.1 - 1 { self.0 } else { self.0 + random.next_int(self.1 - self.0) }
    }

    /// `IntRange::getValueInclusive`: `min + nextInt(max - min + 1)`, no draw when `min >= max`.
    pub fn draw_inclusive(self, random: &mut Mt19937) -> i32 {
        if self.0 >= self.1 { self.0 } else { self.0 + random.next_int(self.1 - self.0 + 1) }
    }
}

impl Odds {
    pub fn roll(self, random: &mut Mt19937) -> bool {
        match self {
            Odds::Fraction(_, 0) => false,
            Odds::Fraction(n, d) if n == d => true,
            Odds::Fraction(n, d) => random.next_u32() % d < n,
            Odds::Percent(p) if p <= 0.0 => false,
            Odds::Percent(p) if p >= 100.0 => true,
            Odds::Percent(p) => random.next_float() * 100.0 < p,
        }
    }
}

/// An allow list; empty allows everything.
pub fn allows(list: &[Block], block: Block) -> bool {
    list.is_empty() || list.contains(&block)
}

/// Plants a tree grows through (`BlockMaterial` replaceable plants).
pub fn is_replaceable_plant(block: Block) -> bool {
    matches!(
        block,
        Block::ShortGrass
            | Block::Fern
            | Block::TallGrass
            | Block::TallGrassTop
            | Block::LargeFern
            | Block::LargeFernTop
            | Block::Deadbush
            | Block::Vine1
            | Block::Vine2
            | Block::Vine4
            | Block::Vine8
    )
}

/// The canopies' leaf gate: air, leaves or a replaceable plant (`may_replace` is not asked).
pub fn leaf_replaceable(block: Block) -> bool {
    block == Block::Air || block.is_leaves() || is_replaceable_plant(block)
}

/// `TreeHelper::isValidTreePosition`: `may_replace`, or air, leaves, water or a replaceable plant.
pub fn valid_tree_position(view: &View, [x, y, z]: [i32; 3], may_replace: &[Block]) -> bool {
    let block = view.block(x, y, z);
    allows(may_replace, block) || leaf_replaceable(block) || block == Block::Water
}

impl Tree {
    /// The ground under a trunk cell becomes the first base block unless it is one already.
    pub fn place_base_block(&self, view: &mut View, [x, y, z]: [i32; 3]) {
        let Some(&first) = self.base_block.first() else { return };
        if !self.base_block.contains(&view.block(x, y, z)) {
            view.set_block(x, y, z, first);
        }
    }

    /// `TreeHelper::prepareSpawn` (1438eac70): inside the world, `may_grow_on` under the footprint,
    /// and every cell of the trunk's volume (radius 0 at the base, 1 above, 2 from one under the
    /// top to one over it) growable through.
    pub fn prepare(&self, view: &View, [x, y, z]: [i32; 3], height: i32, width: i32) -> bool {
        let top = crate::terrain::MIN_Y + crate::terrain::HEIGHT as i32;
        if y <= crate::terrain::MIN_Y || y + height >= top {
            return false;
        }
        if !(0..width).all(|dx| (0..width).all(|dz| allows(self.may_grow_on, view.block(x + dx, y - 1, z + dz)))) {
            return false;
        }
        // The list BDS checks behaves as may_grow_through plus may_replace (oracle: trees grow through air and leaves, not
        // logs); without may_grow_through only air and water pass (oracle: swamp oaks fail under leaves).
        let through = |block: Block| match self.may_grow_through.is_empty() {
            true => matches!(block, Block::Air | Block::Water),
            false => self.may_grow_through.contains(&block) || allows(self.may_replace, block),
        };
        (y..=(y + height + 1).min(top - 1)).all(|cy| {
            let r = if cy >= y + height - 1 { 2 } else { i32::from(cy != y) };
            (x - r..=x + r).all(|cx| (z - r..=z + r).all(|cz| through(view.block(cx, cy, cz))))
        })
    }
}

/// The sides an attachable decoration tries, in BDS's order: west, east, north, south.
pub const SIDES: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

/// A vine hung `offset` away from the block it clings to: its `vine_direction_bits` face that block.
pub fn vine_facing((dx, dz): (i32, i32)) -> Block {
    match (dx, dz) {
        (-1, _) => Block::Vine8,
        (1, _) => Block::Vine2,
        (_, -1) => Block::Vine1,
        _ => Block::Vine4,
    }
}

impl Decorative {
    fn block(self, side: (i32, i32)) -> Block {
        match self {
            Decorative::Vine => vine_facing(side),
            Decorative::Block(block) => block,
        }
    }
}

impl Decoration {
    /// Per enabled side one chance roll, then each entry's count (drawn even when the cell is
    /// taken) and a run of it while the cells are air.
    pub fn place(&self, view: &mut View, [x, y, z]: [i32; 3], enabled: [bool; 4], random: &mut Mt19937) {
        for (i, &side) in SIDES.iter().enumerate() {
            if !enabled[i] || !self.chance.roll(random) {
                continue;
            }
            let mut p = [x + side.0, y, z + side.1];
            if view.block(p[0], p[1], p[2]) != Block::Air {
                continue;
            }
            let step = match self.step {
                Step::Outward => [side.0, 0, side.1],
                Step::Up => [0, 1, 0],
                Step::Down => [0, -1, 0],
            };
            for &(block, count) in self.entries {
                let count = count.draw_inclusive(random);
                if view.block(p[0], p[1], p[2]) != Block::Air || count < 1 {
                    continue;
                }
                for n in 0..count {
                    view.set_block(p[0], p[1], p[2], block.block(side));
                    p = [p[0] + step[0], p[1] + step[1], p[2] + step[2]];
                    if n + 1 < count && view.block(p[0], p[1], p[2]) != Block::Air {
                        break;
                    }
                }
            }
        }
    }

    /// `placeDecoratedBlock`: the block, then the decoration on all four sides if it was written.
    pub fn place_decorated(decoration: Option<&Decoration>, view: &mut View, [x, y, z]: [i32; 3], block: Block, random: &mut Mt19937) -> bool {
        let placed = view.set_block(x, y, z, block);
        if placed && let Some(decoration) = decoration {
            decoration.place(view, [x, y, z], [true; 4], random);
        }
        placed
    }
}

impl CanopyDecoration {
    /// Per side of a leaf one chance roll, then a vine run downwards `steps + 1` long while the
    /// cells are air.
    pub fn place(&self, view: &mut View, [x, y, z]: [i32; 3], random: &mut Mt19937) {
        for side in SIDES {
            if !self.chance.roll(random) {
                continue;
            }
            let (px, pz) = (x + side.0, z + side.1);
            if view.block(px, y, pz) != Block::Air {
                continue;
            }
            let block = self.block.block(side);
            let steps = self.steps.draw_inclusive(random) + 1;
            let mut py = y;
            for _ in 0..steps {
                if view.block(px, py, pz) != Block::Air {
                    break;
                }
                view.set_block(px, py, pz, block);
                py -= 1;
            }
        }
    }
}

impl Odds {
    /// `ChanceInformation::isValid`: a fraction needs a denominator, a percent must not be negative.
    pub fn is_valid(self) -> bool {
        match self {
            Odds::Fraction(_, d) => d != 0,
            Odds::Percent(p) => p >= 0.0,
        }
    }
}

/// The random-spread and mangrove canopies' leaf pick: `nextInt` of the truncated weight sum (no
/// draw when 0), then the first entry the remainder, truncated after each step, goes negative at.
pub fn pick_weighted(blocks: &[(Block, i32)], random: &mut Mt19937) -> Block {
    let sum: i32 = blocks.iter().map(|&(_, weight)| weight).sum();
    let mut left = if sum != 0 { random.next_int(sum) } else { 0 };
    for &(block, weight) in blocks {
        left -= weight;
        if left < 0 {
            return block;
        }
    }
    blocks[blocks.len() - 1].0
}

/// A log lying along the dominant horizontal component of `(dx, dz)` (x on ties), upright when
/// both are 0; logs without x/z variants stay upright.
pub fn log_along(log: Block, dx: i32, dz: i32) -> Block {
    let [up, x, z] = match log {
        Block::OakLog => [Block::OakLog, Block::OakLogX, Block::OakLogZ],
        Block::SpruceLog => [Block::SpruceLog, Block::SpruceLogX, Block::SpruceLogZ],
        Block::BirchLog => [Block::BirchLog, Block::BirchLogX, Block::BirchLogZ],
        Block::JungleLog => [Block::JungleLog, Block::JungleLogX, Block::JungleLogZ],
        Block::CherryLog => [Block::CherryLog, Block::CherryLogX, Block::CherryLogZ],
        _ => return log,
    };
    match (dx.abs(), dz.abs()) {
        (0, 0) => up,
        (ax, az) if ax >= az => x,
        _ => z,
    }
}

impl Tree {
    /// A cell a trunk may grow through: the same rule as `prepare`'s volume check.
    pub fn grows_through(&self, block: Block) -> bool {
        self.may_grow_through.contains(&block) || allows(self.may_replace, block)
    }
}
