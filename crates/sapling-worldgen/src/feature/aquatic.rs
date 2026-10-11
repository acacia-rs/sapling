//! The hardcoded underwater plants: `KelpFeature::place` (149e915a0) and `SeagrassFeature::place`
//! (149d61510). Both find a floor under the position, then try `count` columns: climb to the first
//! water or air, place, and move to a new column ±7 around the floor (z drawn before x).

use super::place::Ctx;
use super::view::View;
use crate::block::{Block, KELP};
use crate::random::Mt19937;
use crate::terrain::{HEIGHT, MIN_Y};

/// Kelp's state count: columns are measured up to this many water blocks, and the top block of
/// a column gets age `KELP_AGES - 2`.
const KELP_AGES: i32 = KELP.len() as i32;

/// Biomes colder than 0.15 (`minecraft:climate` temperature) among the overworld's.
fn is_cold(biome: u32) -> bool {
    matches!(biome, 11 | 12 | 26 | 30 | 46 | 140 | 182 | 183 | 184 | 185)
}

/// `isSolidBlockingBlock` (vtable 0x1b0): leaves are not (oracle: seagrass reaches the water under
/// mangrove canopies).
pub(super) fn solid(view: &View, x: i32, y: i32, z: i32) -> bool {
    let block = view.block(x, y, z);
    // A structure's fences and stairs are not (oracle: kelp under a shipwreck's deck).
    let full = !matches!(block, Block::Template(_)) || crate::structure::template::flags(block) & crate::structure::template::FULL_CUBE != 0;
    block.blocks_motion() && !block.is_leaves() && full
}

/// Up from `p` to the first water or air block (seagrass and kelp are not water: oracle).
fn climb(view: &View, p: &mut [i32; 3]) {
    while p[1] < MIN_Y + HEIGHT as i32 && !matches!(view.block(p[0], p[1], p[2]), Block::Water | Block::Air) {
        p[1] += 1;
    }
}

/// The next column, ±7 around `base`: two raw draws masked to 0..7 per axis, z first.
fn next_column(random: &mut Mt19937, base: [i32; 3]) -> [i32; 3] {
    let mut offset = || (random.next_u32() & 7) as i32 - (random.next_u32() & 7) as i32;
    let dz = offset();
    let dx = offset();
    [base[0] + dx, base[1], base[2] + dz]
}

pub fn kelp(ctx: &mut Ctx, at: [i32; 3], count: i32) -> bool {
    let view = &mut *ctx.view;
    let [x, mut y, z] = at;
    while !solid(view, x, y - 1, z) && !view.block(x, y, z).is_kelp() && y > MIN_Y {
        y -= 1;
    }
    if !solid(view, x, y - 1, z) {
        return false;
    }
    let base = [x, y, z];
    let mut p = base;
    for _ in 0..count {
        climb(view, &mut p);
        let [px, py, pz] = p;
        // Without a full column the try neither places nor moves on, so later tries repeat it.
        if view.block(px, py, pz) == Block::Air || view.block(px, py - 1, pz) == Block::Water {
            continue;
        }
        let depth = (0..KELP_AGES).take_while(|&h| view.block(px, py + h, pz) == Block::Water).count() as i32;
        if depth < 3 {
            continue;
        }
        let length = if depth > 3 { ctx.random.next_int(depth - 3) + 3 } else { 3 };
        let below = view.block(px, py - 1, pz);
        // `KelpBlock::canSurvive` (142fdc7b0): a solid-blocking block below, or a slab or stair turned over
        // (a shipwreck's upright stairs hold none).
        let turned = |b: Block| b.states().contains("upside_down_bit=1") || b.states().contains("vertical_half=top");
        if !below.is_kelp() && (below.is_solid_blocking() || turned(below)) {
            let age = KELP_AGES - length - 1;
            view.set_block(px, py, pz, KELP[age as usize]);
            for i in 1..length {
                if view.block(px, py + i, pz) != Block::Water {
                    break;
                }
                view.set_block(px, py + i, pz, KELP[(age + i) as usize]);
            }
        }
        p = next_column(ctx.random, base);
    }
    true
}

pub fn seagrass(ctx: &mut Ctx, at: [i32; 3], count: i32, tall: f32) -> bool {
    let cold = (ctx.biome)(at).is_some_and(is_cold);
    let view = &mut *ctx.view;
    let [x, mut y, z] = at;
    while !solid(view, x, y, z) && y > MIN_Y {
        y -= 1;
    }
    if !solid(view, x, y, z) {
        return false;
    }
    let base = [x, y, z];
    let mut p = base;
    for _ in 0..count {
        climb(view, &mut p);
        let [px, py, pz] = p;
        let water: [bool; 3] = std::array::from_fn(|dy| view.block(px, py + dy as i32, pz) == Block::Water);
        // The float is drawn only once the seagrass may stand here (oracle: drawing it always desyncs).
        if water[0] && solid(view, px, py - 1, pz) {
            if tall > ctx.random.next_float() {
                if water[1] && (!cold || water[2]) {
                    view.set_block(px, py, pz, Block::SeagrassBottom);
                    view.set_block(px, py + 1, pz, Block::SeagrassTop);
                }
            } else if !cold || water[1] {
                view.set_block(px, py, pz, Block::Seagrass);
            }
        }
        p = next_column(ctx.random, base);
    }
    true
}

/// `SeaPickleFeature::place` (149d61020): down to a solid block, then four tries at columns
/// 0..5 off it (z drawn before x), each a random cluster count on the first water above.
pub fn sea_pickle(view: &mut View, random: &mut Mt19937, at: [i32; 3]) -> bool {
    let [x, mut y, z] = at;
    while !solid(view, x, y, z) && !view.block(x, y, z).is_sea_pickle() && y > MIN_Y {
        y -= 1;
    }
    if !solid(view, x, y, z) {
        return false;
    }
    for _ in 0..4 {
        let dz = random.next_int(6);
        let dx = random.next_int(6);
        let count = random.next_int(4);
        let mut p = [x + dx, y, z + dz];
        climb(view, &mut p);
        let [px, py, pz] = p;
        if view.block(px, py, pz) == Block::Water && solid(view, px, py - 1, pz) {
            view.set_block(px, py, pz, Block::sea_pickle(count));
        }
    }
    true
}
