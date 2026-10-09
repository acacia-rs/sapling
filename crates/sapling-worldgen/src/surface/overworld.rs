//! The default (`minecraft:overworld`) surface builder, 145809ef0: Java 1.17's
//! `DefaultSurfaceBuilder` run from the column top down to the preliminary surface minus 8.

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

/// Sand over air or fluid turns to sandstone, red sand to red sandstone, gravel to the foundation.
fn prop_up_floating_sand(column: &mut Column, y: i32, foundation: Block) {
    let replacement = match column.get(y) {
        Block::Sand => Block::Sandstone,
        Block::RedSand => Block::RedSandstone,
        Block::Gravel => foundation,
        _ => return,
    };
    column.set(y, replacement);
}

/// The surface depth BDS draws once per column, before anything else of the builder.
pub fn surface_depth(random: &mut Mt19937, noise: f32) -> i32 {
    (random.next_float() * 0.25 + noise / 3.0 + 3.0) as i32
}

/// `materials` are already adjusted; a steep slope's material replaces top and mid at every block.
pub fn build(column: &mut Column, materials: Materials, steep: Option<Block>, sea_floor_depth: i32, temperature: f32, depth: i32, random: &mut Mt19937) {
    let materials = match steep {
        Some(steep) => Materials { top: steep, mid: steep, ..materials },
        None => materials,
    };
    // The sea level is the top of the last fluid run above, reset by air (BDS's 0x800003e8).
    let mut sea = NO_SEA;
    let sea_floor_limit = sea_floor_depth + depth;
    let (mut top_material, mut under_material) = (materials.top, materials.mid);
    let mut run = -1;
    let mut y = column.start;
    while y > column.lower {
        let block = column.get(y);
        if run >= 0 && matches!(block, Block::Air | Block::Water | Block::Lava) {
            prop_up_floating_sand(column, y + 1, materials.foundation);
        }
        if steep.is_some() {
            (top_material, under_material) = (materials.top, materials.mid);
        }
        match block {
            Block::Bedrock => {}
            Block::Air => (run, sea) = (-1, NO_SEA),
            _ if REPLACEABLE.contains(&block) => {
                if run == -1 {
                    let (mut top, under) = if depth <= 0 {
                        (Block::Air, materials.foundation)
                    } else if y >= sea - 4 && y <= sea + 1 {
                        (materials.top, materials.mid)
                    } else {
                        (top_material, under_material)
                    };
                    if y < sea && top == Block::Air {
                        top = if materials.sea == Block::Water && temperature < 0.15 { Block::Ice } else { materials.sea };
                    }
                    run = depth;
                    if y < sea - 1 {
                        if y == column.lower + 1 || y < sea - sea_floor_limit {
                            column.set(y, materials.sea_floor);
                            (top_material, under_material) = (Block::Air, materials.foundation);
                            y -= 1;
                            continue;
                        }
                        column.set(y, under);
                    } else {
                        column.set(y, top);
                    }
                    (top_material, under_material) = (top, under);
                } else if run == 0 {
                    column.set(y, materials.foundation);
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
                    column.set(y, materials.sea);
                }
                sea = sea.max(y + 1);
            }
            // An iceberg is sea too (14580ae5c: packed ice and blue ice by id): gravel under it, not dirt.
            Block::PackedIce | Block::BlueIce => sea = sea.max(y + 1),
            _ => {}
        }
        y -= 1;
    }
}
