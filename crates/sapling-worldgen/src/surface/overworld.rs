//! The default (`minecraft:overworld`) surface builder, 145809ef0: Java 1.17's
//! `DefaultSurfaceBuilder` run from the column top down to its lower bound (the overworld's
//! preliminary surface minus 8, the Nether's 0).

use super::column::Column;
use super::spec::Materials;
use crate::block::Block;
use crate::random::Mt19937;

/// Blocks the builder may replace (0x14ccd33e8: stone, granite, diorite, andesite and the polished
/// three); of those, only stone and vein granite exist before features.
const REPLACEABLE: &[Block] = &[Block::Stone, Block::Granite];
/// Under-materials that end in a sandstone run (0x14ccd9470: sand, red sand, suspicious sand).
const SANDS: &[Block] = &[Block::Sand, Block::RedSand];

const NO_SEA: i32 = 0x8000_03e8_u32 as i32;

/// Sand over air or fluid turns to sandstone, red sand to red sandstone, gravel to the foundation;
/// not under a netherrack foundation (14580a409).
fn prop_up_floating_sand(column: &mut Column, y: i32, foundation: Block) {
    let replacement = match column.get(y) {
        Block::Sand => Block::Sandstone,
        Block::RedSand => Block::RedSandstone,
        Block::Gravel => foundation,
        _ => return,
    };
    if foundation != Block::Netherrack {
        column.set(y, replacement);
    }
}

/// The surface depth BDS draws once per column, before anything else of the builder.
pub fn surface_depth(random: &mut Mt19937, noise: f32) -> i32 {
    // 14580a95d: the noise third and the 3 are summed first; the other order rounds another way once in 10^5 columns.
    (random.next_float() * 0.25 + (noise / 3.0 + 3.0)) as i32
}

/// How the builder knows the sea level: the overworld's is the top of the last fluid run above, reset by air
/// (BDS's 0x800003e8); the Nether passes a fixed one (`loadChunk`'s parameter +0x10).
#[derive(Clone, Copy)]
pub enum Sea {
    Tracked,
    Fixed(i32),
}

/// `materials` at a y are already adjusted; a steep slope's material replaces top and mid at every block.
#[allow(clippy::too_many_arguments)]
pub fn build(column: &mut Column, materials: impl Fn(i32) -> Materials, steep: Option<Block>, sea_floor_depth: i32, temperature: f32, depth: i32, sea_mode: Sea, random: &mut Mt19937) {
    let materials = |y: i32| match steep {
        Some(steep) => Materials { top: steep, mid: steep, ..materials(y) },
        None => materials(y),
    };
    let mut sea = match sea_mode {
        Sea::Tracked => NO_SEA,
        Sea::Fixed(level) => level,
    };
    let sea_floor_limit = sea_floor_depth + depth;
    let start = materials(column.start);
    let (mut top_material, mut under_material) = (start.top, start.mid);
    let mut run = -1;
    let mut y = column.start;
    while y > column.lower {
        let block = column.get(y);
        let at = materials(y);
        if run >= 0 && matches!(block, Block::Air | Block::Water | Block::Lava) {
            prop_up_floating_sand(column, y + 1, at.foundation);
        }
        if steep.is_some() {
            (top_material, under_material) = (at.top, at.mid);
        }
        let tracked = matches!(sea_mode, Sea::Tracked);
        match block {
            Block::Bedrock => {}
            Block::Air => {
                run = -1;
                if tracked {
                    sea = NO_SEA;
                }
            }
            _ if REPLACEABLE.contains(&block) => {
                if run == -1 {
                    let (mut top, under) = if depth <= 0 {
                        (Block::Air, at.foundation)
                    } else if y >= sea - 4 && y <= sea + 1 {
                        (at.top, at.mid)
                    } else {
                        (top_material, under_material)
                    };
                    if y < sea && top == Block::Air {
                        top = if at.sea == Block::Water && temperature < 0.15 { Block::Ice } else { at.sea };
                    }
                    run = depth;
                    if y < sea - 1 {
                        if y == column.lower + 1 || y < sea - sea_floor_limit {
                            column.set(y, at.sea_floor);
                            (top_material, under_material) = (Block::Air, at.foundation);
                            y -= 1;
                            continue;
                        }
                        column.set(y, under);
                    } else {
                        column.set(y, top);
                    }
                    (top_material, under_material) = (top, under);
                } else if run == 0 {
                    column.set(y, at.foundation);
                } else {
                    column.set(y, under_material);
                    run -= 1;
                    if run == 0 && SANDS.contains(&under_material) {
                        run = random.next_int(4) + (y - sea).max(0);
                        under_material = Block::Sandstone;
                    }
                }
            }
            Block::Water | Block::Lava => {
                if block == Block::Water {
                    column.set(y, at.sea);
                }
                if tracked {
                    sea = sea.max(y + 1);
                }
            }
            // An iceberg is sea too (14580ae5c: packed ice and blue ice by id): gravel under it, not dirt.
            Block::PackedIce | Block::BlueIce if tracked => sea = sea.max(y + 1),
            _ => {}
        }
        y -= 1;
    }
}
