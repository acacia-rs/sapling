//! The lush caves features BDS builds in code (registered in 1482f4580): moss vegetation (149d99710),
//! the dripleaf (148299bc0), single-face vines (149d9a0f0) and the azalea root system (149d660d0).
//! The first two were fitted to the oracle's placing-generator values and agree with their decompiles'
//! draws; their survival tests are still plants.rs's.

use super::cave::pick_weighted;
use super::data::FEATURES;
use super::place::Ctx;
use super::plants::can_survive;
use super::tree::common::vine_facing;
use super::view::View;
use crate::block::{Block, Support};

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

/// What a vine hangs on (149d9a0f0): a solid-blocking block that blocks motion.
pub fn holds_vine(block: Block) -> bool {
    block.is_solid_blocking() && block.blocks_motion()
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
                // 149d99710: air over a top face that holds, before the block's own `mayPlace`.
                let open = ctx.view.block(at[0], at[1], at[2]) == Block::Air;
                let held = ctx.view.block(at[0], at[1] - 1, at[2]).supports(1, Support::Any);
                (open && held && double_or_single(ctx.view, at, block)).then_some(at)
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
    // 149d9a0f0's side order: north, east, south, west.
    for (dx, dz) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
        if holds_vine(view.block(x + dx, y, z + dz)) {
            return view.set_block(x, y, z, vine_facing((-dx, -dz)));
        }
    }
    false
}

/// How many cells up the column a tree position is looked for (149d660d0).
const ROOT_COLUMN_MAX: i32 = 184;
const ATTEMPTS: i32 = 20;

/// `RootSystemFeature::place` (149d660d0): up the column to the light heightmap, the tree at the first
/// cell over solid ground with three open cells (air or water, not all water) where it grows; then
/// rooted dirt around each cell of the column and hanging roots on a walk from the start.
fn root_system(ctx: &mut Ctx, at: [i32; 3], tree: usize) -> Option<[i32; 3]> {
    let [x, y, z] = at;
    super::trace::line(|| format!("  roots {x} {y} {z} draw {}", ctx.random.draws()));
    let top = ctx.view.heightmap(x, z);
    let water = |view: &View, y: i32| view.block(x, y, z).is_fluid() && !view.block(x, y, z).is_lava();
    let open = |view: &View, y: i32| view.block(x, y, z) == Block::Air || water(view, y);
    let mut tree_at = None;
    for ty in y..y + ROOT_COLUMN_MAX {
        let roomy = (0..3).all(|d| open(ctx.view, ty + d)) && !(0..3).all(|d| water(ctx.view, ty + d));
        if ctx.view.block(x, ty - 1, z).is_solid() && roomy {
            super::trace::line(|| format!("  tree {x} {ty} {z} draw {}", ctx.random.draws()));
            if FEATURES[tree].place(ctx, [x, ty, z]).is_some() {
                tree_at = Some(ty);
                break;
            }
        }
        if ty >= top {
            break;
        }
    }
    for cy in y + 1..=tree_at? {
        for _ in 0..ATTEMPTS {
            let dx = jitter(ctx, 3);
            let dz = jitter(ctx, 3);
            if root_replaceable(ctx.view.block(x + dx, cy, z + dz)) {
                ctx.view.set_block(x + dx, cy, z + dz, Block::DirtWithRoots);
            }
        }
    }
    // The offsets add up: BDS never returns to the start (x and z drawn before y).
    let mut p = at;
    for _ in 0..ATTEMPTS {
        p[0] += jitter(ctx, 3);
        p[2] += jitter(ctx, 3);
        p[1] += jitter(ctx, 2);
        if ctx.view.block(p[0], p[1], p[2]) == Block::Air && ctx.view.block(p[0], p[1] + 1, p[2]).is_solid() {
            ctx.view.set_block(p[0], p[1], p[2], Block::HangingRoots);
        }
    }
    Some(at)
}

fn jitter(ctx: &mut Ctx, bound: i32) -> i32 {
    let a = ctx.random.next_int(bound);
    a - ctx.random.next_int(bound)
}

/// The root system's replaceable list (VanillaBlockTypeIds 14ccd2758, 17 blocks).
fn root_replaceable(block: Block) -> bool {
    use Block::*;
    matches!(block, Clay | MossBlock | Sand | Gravel | Dirt | Podzol | DirtWithRoots | GrassBlock | Mycelium | Stone | Deepslate | Tuff | ShortGrass | Fern)
        || matches!(block.name(), "cave_vines" | "cave_vines_head_with_berries") || block == MossCarpet
}
