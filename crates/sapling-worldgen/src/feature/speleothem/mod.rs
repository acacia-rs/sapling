//! Bedrock's speleothem features (registered in 1482f4580), each for pointed dripstone on dripstone
//! blocks and for sulfur spikes on sulfur: `SpeleothemFeature::place` (149d62d10), the cluster
//! (`SpeleothemClusterFeature`, 149d61dc0, cluster.rs) and the large one (149e94510, large.rs).
//! Java 1.18's dripstone features with Bedrock's own draws and block tests.

mod cluster;
mod large;

use super::place::Ctx;
use super::view::View;
use crate::block::Block;

/// The blocks a speleothem family grows from and replaces (VanillaBlockTypeIds lists 14ccd2770,
/// 14ccd27a0).
#[derive(Debug)]
pub struct SpeleothemBlocks {
    pub base: Block,
    /// The standing tip of the spike block (`Block::speleothem`).
    pub spike: Block,
    pub replaceable: &'static [Block],
}

pub static DRIPSTONE: SpeleothemBlocks = SpeleothemBlocks {
    base: Block::DripstoneBlock,
    spike: Block::PointedDripstone,
    replaceable: &[Block::Dirt, Block::CoarseDirt, Block::Stone, Block::Tuff, Block::Deepslate],
};

pub static SULFUR: SpeleothemBlocks =
    SpeleothemBlocks { base: Block::Sulfur, spike: Block::SulfurSpike, replaceable: &[Block::Sulfur, Block::Cinnabar] };

#[derive(Debug)]
pub enum Speleothem {
    /// `pointed_dripstone_feature`, `sulfur_spike_feature`.
    Pointed(&'static SpeleothemBlocks),
    /// `dripstone_cluster_feature`, `sulfur_spike_cluster_feature`: column height range and the
    /// wetness clamped normal (mean, deviation, min, max).
    Cluster { blocks: &'static SpeleothemBlocks, height: [i32; 2], wetness: [f32; 4] },
    /// `large_dripstone_feature`.
    Large(&'static SpeleothemBlocks),
}

impl Speleothem {
    pub fn place(&self, ctx: &mut Ctx, at: [i32; 3]) -> Option<[i32; 3]> {
        match *self {
            Speleothem::Pointed(blocks) => pointed(ctx, blocks, at),
            Speleothem::Cluster { blocks, height, wetness } => cluster::place(ctx, blocks, height, wetness, at),
            Speleothem::Large(blocks) => large::place(ctx, blocks, at),
        }
    }
}

/// FacingID offsets: down, up, north, south, west, east.
const FACING: [[i32; 3]; 6] = [[0, -1, 0], [0, 1, 0], [0, 0, -1], [0, 0, 1], [-1, 0, 0], [1, 0, 0]];

fn step([x, y, z]: [i32; 3], facing: usize, n: i32) -> [i32; 3] {
    let [dx, dy, dz] = FACING[facing];
    [x + dx * n, y + dy * n, z + dz * n]
}

/// Water in the block or its second layer (material 5).
fn water(block: Block) -> bool {
    (block.is_fluid() && !block.is_lava()) || block.is_waterlogged()
}

fn empty_or_water(block: Block) -> bool {
    block == Block::Air || water(block) && (crate::xp(1) || !block.is_wet_speleothem())
}

fn get(view: &View, [x, y, z]: [i32; 3]) -> Block {
    view.block(x, y, z)
}

impl SpeleothemBlocks {
    fn is_base(&self, block: Block) -> bool {
        self.replaceable.contains(&block) || block == self.base
    }

    /// The base block over a replaceable one (flags 3); whether it was replaceable.
    fn replace(&self, view: &mut View, [x, y, z]: [i32; 3]) -> bool {
        let fits = self.replaceable.contains(&view.block(x, y, z));
        if fits {
            view.set_block(x, y, z, self.base);
        }
        fits
    }

    /// `growPointedDripstone` (14829e980): `height` spikes from `at` along `up`, base first and the
    /// tip last, each into air or water only and never lava.
    fn grow(&self, view: &mut View, at: [i32; 3], up: bool, height: i32, merge: bool) {
        let facing = usize::from(up);
        for i in 0..height.max(0) {
            let p = step(at, facing, i);
            let block = get(view, p);
            if !empty_or_water(block) || block.is_lava() {
                return;
            }
            let left = height - i;
            let thickness = match left {
                1 if merge => 4,
                1 => 0,
                2 => 1,
                _ if i == 0 => 3,
                _ => 2,
            };
            view.set_block(p[0], p[1], p[2], self.spike.speleothem(thickness, !up));
        }
    }
}

/// `SpeleothemFeature::place`: the tip direction from the bases above and below (both: a
/// boolean), a patch of base blocks behind it, then one or two spikes.
fn pointed(ctx: &mut Ctx, blocks: &SpeleothemBlocks, at: [i32; 3]) -> Option<[i32; 3]> {
    crate::feature::trace::line(|| format!("  pointed {} {} {} draw {}", at[0], at[1], at[2], ctx.random.draws()));
    let below = blocks.is_base(get(ctx.view, step(at, 0, 1)));
    let above = blocks.is_base(get(ctx.view, step(at, 1, 1)));
    let up = match (below, above) {
        (true, true) => ctx.random.next_boolean(),
        (true, false) => true,
        (false, true) => false,
        (false, false) => return None,
    };
    let root = step(at, usize::from(!up), 1);
    blocks.replace(ctx.view, root);
    for facing in 0..6 {
        if ctx.random.next_float() < 0.3 {
            continue;
        }
        let near = step(root, facing, 1);
        blocks.replace(ctx.view, near);
        if ctx.random.next_boolean() {
            continue;
        }
        let far = step(near, ctx.random.next_int(6) as usize, 1);
        blocks.replace(ctx.view, far);
        if ctx.random.next_boolean() {
            continue;
        }
        let farthest = step(far, ctx.random.next_int(6) as usize, 1);
        blocks.replace(ctx.view, farthest);
    }
    let taller = ctx.random.next_float() < 0.2 && empty_or_water(get(ctx.view, step(at, usize::from(up), 1)));
    blocks.grow(ctx.view, at, up, if taller { 2 } else { 1 }, false);
    Some(at)
}
