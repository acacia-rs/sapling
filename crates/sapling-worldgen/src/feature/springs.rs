//! `legacy:springs_feature` (vtable 14a98e800, place 149e97230): 25 water and 20 lava tries per
//! chunk, each a `SpringFeature::place` (149d65900): Java's legacy liquid spring, a source set into
//! a wall with stone above, below and on three sides and air on the fourth, then ticked.

use super::place::Ctx;
use super::view::View;
use crate::block::Block;
use crate::terrain::MIN_Y;

const WATER_TRIES: usize = 25;
const LAVA_TRIES: usize = 20;
/// The world's lowest y plus this bounds the water tries (`0x80 + 0x40` above the minimum).
const WATER_RANGE: i32 = 256;
const LAVA_RANGE: i32 = 240;
const LAVA_FLOOR: i32 = MIN_Y + 8;
/// Where lava tries are skipped: deep dark (by id) and the biomes tagged frozen and ocean, or
/// ice_plains and mutated (tag table offsets 0x128 & 0x98, 0x218 & 0x1d0: VanillaBiomeTags
/// 14393d890 order at 0x18 each from ocean at 0x98): frozen oceans and ice spikes.
const NO_LAVA: [u32; 4] = [190, 46, 47, 140];

pub fn springs(ctx: &mut Ctx, [x0, _, z0]: [i32; 3]) -> bool {
    for _ in 0..WATER_TRIES {
        let z = ctx.random.next_int(16);
        let y = ctx.random.next_int(WATER_RANGE) + MIN_Y;
        let x = ctx.random.next_int(16);
        spring(ctx.view, [x0 + x, y, z0 + z], false);
    }
    for _ in 0..LAVA_TRIES {
        let z = ctx.random.next_int(16);
        let bound = ctx.random.next_int(LAVA_RANGE);
        let bound = ctx.random.next_int(bound);
        let y = ctx.random.next_int(bound) + LAVA_FLOOR;
        let x = ctx.random.next_int(16);
        let at = [x0 + x, y, z0 + z];
        if (ctx.biome)(at).is_none_or(|biome| !NO_LAVA.contains(&biome)) {
            spring(ctx.view, at, true);
        }
    }
    true
}

/// 149d65e10: stone, or in current worlds one of the blocks Java's spring accepts as rock.
fn rock(block: Block) -> bool {
    use Block::*;
    matches!(block, Stone | Deepslate | Tuff | Calcite | Snow | PowderSnow | PackedIce | Dirt | CoarseDirt)
}

fn spring(view: &mut View, [x, y, z]: [i32; 3], lava: bool) {
    let at = |dx: i32, dy: i32, dz: i32| view.block(x + dx, y + dy, z + dz);
    if !rock(at(0, 1, 0)) || !rock(at(0, -1, 0)) || !(at(0, 0, 0) == Block::Air || rock(at(0, 0, 0))) {
        return;
    }
    let around = [at(-1, 0, 0), at(1, 0, 0), at(0, 0, -1), at(0, 0, 1), at(0, -1, 0)];
    let rocks = around.iter().filter(|&&b| rock(b)).count();
    let air = around.iter().filter(|&&b| b == Block::Air).count();
    if rocks == 4 && air == 1 {
        view.set_block(x, y, z, Block::liquid(lava, 0));
        view.schedule([x, y, z], lava, 1);
    }
}
