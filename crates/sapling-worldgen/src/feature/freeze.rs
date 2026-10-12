//! The freeze pass that ends `decorationPostProcessChunk` (1482143a0): per column of the decorated
//! chunk, x outer and z inner, water under the rain top turns to ice (`Biome::shouldFreeze`
//! 14119e860) and the rain top gets a snow layer (`Biome::shouldSnow` 14119e740), both by
//! `Biome::getTemperature` (surface/temperature.rs).

use super::view::View;
use crate::biome::ChunkBiomes;
use crate::block::Block;
use crate::surface::temperature;
use crate::terrain::{HEIGHT, MIN_Y};

fn cold(biomes: &ChunkBiomes, at: [i32; 3]) -> bool {
    if !(MIN_Y..MIN_Y + HEIGHT as i32).contains(&at[1]) {
        return false;
    }
    let biome = biomes.biome(at[0].rem_euclid(16) as usize, at[1], at[2].rem_euclid(16) as usize);
    temperature(biome, at).is_some_and(|t| t <= 0.15)
}

/// One above the highest block that stops rain (140a09950): anything but air and plants.
fn rain_top(view: &View, x: i32, z: i32) -> i32 {
    (MIN_Y..MIN_Y + HEIGHT as i32).rev().find(|&y| stops_rain(view.block(x, y, z))).map_or(MIN_Y, |y| y + 1)
}

fn stops_rain(block: Block) -> bool {
    block != Block::Air && (!block.is_plant() || block.is_crop_block())
}

/// `TopSnowBlock::mayPlace` (1471fc4a0) of the block below: leaves (material 7) hold a layer; ice, frosted ice and
/// barriers (materials 13, 19) and fences, walls, gates, cauldrons and composters (property bits 0x10000050) do
/// not; the rest by the full support of the top face (142cd6d20).
fn holds_snow(block: Block) -> bool {
    let name = block.name();
    block.is_leaves() || !matches!(name, "ice" | "frosted_ice" | "barrier") && !name.ends_with("fence_gate") && block.supports_multiface_plain(1)
}

pub fn freeze_chunk(chunk: (i32, i32), biomes: &ChunkBiomes, view: &mut View) {
    for x in chunk.0 * 16..chunk.0 * 16 + 16 {
        for z in chunk.1 * 16..chunk.1 * 16 + 16 {
            let top = rain_top(view, x, z);
            let below = top - 1;
            if view.block(x, below, z) == Block::Water && cold(biomes, [x, below, z]) {
                view.set_block(x, below, z, Block::Ice);
            }
            // Rain passes plants: the snow lands in a small plant's cell and keeps the plant (block/layers.rs).
            let here = view.block(x, top, z);
            let snow = if here == Block::Air { Some(Block::SnowLayer) } else { here.snowed() };
            if let Some(snow) = snow
                && holds_snow(view.block(x, below, z))
                && cold(biomes, [x, top, z])
            {
                view.set_block(x, top, z, snow);
            }
        }
    }
}
