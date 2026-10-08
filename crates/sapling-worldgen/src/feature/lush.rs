//! The lush caves features BDS builds in code (registered in 1482f4580). None is decompiled yet:
//! each follows its Java analogue (`CaveFeatures`). The draws of the moss vegetation and the
//! dripleaf were fitted to the oracle's placing-generator values; the vines' and the root system's
//! gates and draws are guesses.

use super::cave::pick_weighted;
use super::data::FEATURES;
use super::place::Ctx;
use super::plants::can_survive;
use super::tree::common::vine_facing;
use super::view::View;
use crate::block::Block;
use crate::terrain::MIN_Y;

#[derive(Debug, Clone, Copy)]
pub enum Lush {
    /// `moss_vegetation_feature`: Java's weighted azaleas, moss carpet and grasses.
    MossVegetation,
    /// `dripleaf_feature`: a small dripleaf or a `big_dripleaf_*_feature` (south, west, north,
    /// east), small first, then the facing.
    Dripleaf { big: [usize; 4] },
    /// `vines_single_face_feature`: one vine on the first solid side.
    VinesSingleFace,
    /// `azalea_root_system_feature`: Java's `RootSystemFeature` with `tree`.
    AzaleaRootSystem { tree: usize },
}

const MOSS_VEGETATION: [(Block, f32); 5] =
    [(Block::FloweringAzalea, 4.0), (Block::Azalea, 7.0), (Block::MossCarpet, 25.0), (Block::ShortGrass, 50.0), (Block::TallGrass, 10.0)];
/// By the facing draw: south, west, north, east.
const SMALL_DRIPLEAF: [Block; 4] = [Block::SmallDripleafSouth, Block::SmallDripleafWest, Block::SmallDripleafNorth, Block::SmallDripleafEast];
/// Fitted to the oracle: a first draw under 0.2 picks the small dripleaf.
const SMALL_DRIPLEAF_CHANCE: f32 = 0.2;

/// A full face to stand on or hang from.
pub fn sturdy(block: Block) -> bool {
    block.blocks_motion()
}

/// The waterlogged twin of a dripleaf (the same states, saved with water).
pub fn wet(block: Block) -> Block {
    let offset = Block::BigDripleafSouthWet.index() - Block::BigDripleafSouth.index();
    if (Block::BigDripleafSouth..=Block::SmallDripleafTopEast).contains(&block) { Block::ALL[block.index() + offset] } else { block }
}

impl Lush {
    pub fn place(&self, ctx: &mut Ctx, at: [i32; 3]) -> Option<[i32; 3]> {
        match *self {
            Lush::MossVegetation => {
                let block = pick_weighted(&MOSS_VEGETATION, ctx.random)?;
                double_or_single(ctx.view, at, block).then_some(at)
            }
            Lush::Dripleaf { big } => {
                let small = ctx.random.next_float() < SMALL_DRIPLEAF_CHANCE;
                let facing = ctx.random.next_int(4) as usize;
                if small {
                    double_or_single(ctx.view, at, SMALL_DRIPLEAF[facing]).then_some(at)
                } else {
                    FEATURES[big[facing]].place(ctx, at)
                }
            }
            Lush::VinesSingleFace => vine(ctx.view, at).then_some(at),
            Lush::AzaleaRootSystem { tree } => root_system(ctx, at, tree),
        }
    }
}

/// `SimpleBlockFeature`: the block if it survives, a double plant only with room above; water
/// at either half waterlogs it.
fn double_or_single(view: &mut View, [x, y, z]: [i32; 3], block: Block) -> bool {
    if !can_survive(view, [x, y, z], block) {
        return false;
    }
    let top = match block {
        Block::TallGrass => Some(Block::TallGrassTop),
        b if (Block::SmallDripleafSouth..=Block::SmallDripleafEast).contains(&b) => Some(Block::ALL[b.index() + 4]),
        _ => None,
    };
    let in_water = |view: &View, y| view.block(x, y, z) == Block::Water;
    if let Some(top) = top {
        if !matches!(view.block(x, y + 1, z), Block::Air | Block::Water) {
            return false;
        }
        let top = if in_water(view, y + 1) { wet(top) } else { top };
        let bottom = if in_water(view, y) { wet(block) } else { block };
        view.set_block(x, y, z, bottom);
        view.set_block(x, y + 1, z, top);
        return true;
    }
    view.set_block(x, y, z, block)
}

fn vine(view: &mut View, [x, y, z]: [i32; 3]) -> bool {
    if view.block(x, y, z) != Block::Air {
        return false;
    }
    // Java's side order: north, south, west, east.
    for (dx, dz) in [(0, -1), (0, 1), (-1, 0), (1, 0)] {
        if sturdy(view.block(x + dx, y, z + dz)) {
            return view.set_block(x, y, z, vine_facing((-dx, -dz)));
        }
    }
    false
}

const ROOT_COLUMN_MAX: i32 = 100;
const TREE_SPACE: i32 = 3;
const ROOT_RADIUS: i32 = 3;
const ROOT_ATTEMPTS: i32 = 20;
const HANGING_RADIUS: i32 = 3;
const HANGING_SPAN: i32 = 2;
const HANGING_ATTEMPTS: i32 = 20;

/// Up from the cave ceiling to the first cell with room for the tree and azalea soil under it;
/// the tree there, rooted dirt down the column, hanging roots around the start.
fn root_system(ctx: &mut Ctx, at: [i32; 3], tree: usize) -> Option<[i32; 3]> {
    let [x, y, z] = at;
    if ctx.view.block(x, y, z) != Block::Air {
        return None;
    }
    let mut placed = None;
    for i in 0..ROOT_COLUMN_MAX {
        let ty = y + i + 1;
        if !tree_position(ctx.view, [x, ty, z]) || !(1..=TREE_SPACE).all(|d| ctx.view.block(x, ty + d, z) == Block::Air) {
            continue;
        }
        if !sturdy(ctx.view.block(x, ty - 1, z)) {
            return None;
        }
        if FEATURES[tree].place(ctx, [x, ty, z]).is_some() {
            placed = Some(y + i);
            break;
        }
    }
    let top = placed?;
    for cy in y..top {
        for _ in 0..ROOT_ATTEMPTS {
            let dx = jitter(ctx, ROOT_RADIUS);
            let dz = jitter(ctx, ROOT_RADIUS);
            if root_replaceable(ctx.view.block(x + dx, cy, z + dz)) {
                ctx.view.set_block(x + dx, cy, z + dz, Block::DirtWithRoots);
            }
        }
    }
    for _ in 0..HANGING_ATTEMPTS {
        let dx = jitter(ctx, HANGING_RADIUS);
        let dy = jitter(ctx, HANGING_SPAN);
        let dz = jitter(ctx, HANGING_RADIUS);
        let p = [x + dx, y + dy, z + dz];
        if p[1] > MIN_Y && ctx.view.block(p[0], p[1], p[2]) == Block::Air && sturdy(ctx.view.block(p[0], p[1] + 1, p[2])) {
            ctx.view.set_block(p[0], p[1], p[2], Block::HangingRoots);
        }
    }
    Some(at)
}

fn jitter(ctx: &mut Ctx, bound: i32) -> i32 {
    let a = ctx.random.next_int(bound);
    a - ctx.random.next_int(bound)
}

fn tree_position(view: &View, [x, y, z]: [i32; 3]) -> bool {
    let block = view.block(x, y, z);
    (block == Block::Air || super::tree::common::is_replaceable_plant(block)) && azalea_grows_on(view.block(x, y - 1, z))
}

fn azalea_grows_on(block: Block) -> bool {
    super::plants::is_soil(block) || matches!(block, Block::Sand | Block::RedSand | Block::Clay | Block::Snow | Block::PowderSnow)
}

/// Java's `azalea_root_replaceable`: overworld stone, dirt, sand, clay, gravel and the lush ground.
fn root_replaceable(block: Block) -> bool {
    matches!(
        block,
        Block::Stone | Block::Granite | Block::Diorite | Block::Andesite | Block::Tuff | Block::Deepslate | Block::Sand | Block::RedSand
            | Block::Clay | Block::Gravel | Block::MossBlock | Block::Terracotta | Block::Snow | Block::PowderSnow
    ) || super::plants::is_soil(block) && block != Block::Mud
}
