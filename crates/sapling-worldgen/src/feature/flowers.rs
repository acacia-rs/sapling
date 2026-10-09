//! `LegacyFlowerFeature::place` (149e95670, flower picked by 1438ef3b0) and `MushroomFeature::place`
//! (149d57200): one block type, then `count` tries around the position (land.rs `zyx_offset`).

use super::land::zyx_offset;
use super::molang::noise;
use super::plants::can_survive;
use super::spec::FlowerKind;
use super::view::View;
use crate::block::Block;
use crate::random::Mt19937;

const TULIPS: [Block; 4] = [Block::RedTulip, Block::OrangeTulip, Block::WhiteTulip, Block::PinkTulip];
const PLAINS: [Block; 4] = [Block::Poppy, Block::AzureBluet, Block::OxeyeDaisy, Block::Cornflower];
/// `FlowerFeature::flowerForestFlower` (1438eec60) indexes this list by the noise; the list is
/// filled at load, its order is Java's flower forest list (oracle).
const FLOWER_FOREST: [Block; 11] = [
    Block::Dandelion,
    Block::Poppy,
    Block::Allium,
    Block::AzureBluet,
    Block::RedTulip,
    Block::OrangeTulip,
    Block::WhiteTulip,
    Block::PinkTulip,
    Block::OxeyeDaisy,
    Block::Cornflower,
    Block::LilyOfTheValley,
];

/// The diamond of radius 2 the mushroom checks for shade, in BDS's order.
const SHADE: [[i32; 2]; 13] =
    [[0, 0], [0, -1], [1, 0], [0, 1], [-1, 0], [1, -1], [-1, -1], [1, 1], [-1, 1], [0, -2], [2, 0], [0, 2], [-2, 0]];

/// The flower of one call: nothing is drawn for swamps, one to two draws otherwise.
fn pick(kind: FlowerKind, [x, _, z]: [i32; 3], random: &mut Mt19937) -> Block {
    match kind {
        FlowerKind::Swamp => Block::BlueOrchid,
        FlowerKind::FlowerForest => {
            let value = ((noise().value(x as f32 / 48.0, z as f32 / 48.0) + 1.0) * 0.5).clamp(0.0, 0.9999);
            FLOWER_FOREST[(value * FLOWER_FOREST.len() as f32) as usize]
        }
        FlowerKind::Plains => {
            let value = noise().value(x as f32 / 200.0, z as f32 / 200.0);
            if (value as f64) < -0.8 {
                TULIPS[random.next_int(4) as usize]
            } else if random.next_int(4) > 0 {
                PLAINS[random.next_int(4) as usize]
            } else {
                Block::Dandelion
            }
        }
        FlowerKind::Overworld => {
            if random.next_int(3) == 0 {
                Block::Dandelion
            } else {
                Block::Poppy
            }
        }
    }
}

/// `MeadowFlowerFeature::place` (149e9d1d0, list from its constructor 149e9d080).
const MEADOW: [Block; 8] = [
    Block::Allium,
    Block::Poppy,
    Block::Cornflower,
    Block::OxeyeDaisy,
    Block::AzureBluet,
    Block::Dandelion,
    Block::ShortGrass,
    Block::TallGrass,
];

/// The legacy noise as a value in [0, 1] (`(n + 1) / 2`, unclamped).
fn unit_noise(x: f32, z: f32) -> f32 {
    (noise().value(x, z) + 1.0) * 0.5
}

/// No draws: a slow noise picks how many (1..=4) noise-chosen flowers form the column's palette,
/// a fast one picks among them; placed on air where it survives (tall grass with its top).
pub fn meadow(view: &mut View, [x, y, z]: [i32; 3]) -> bool {
    let slow = unit_noise(x as f32 * 0.0005, z as f32 * 0.0005);
    let variety = if slow < 0.0 { 1 } else if slow > 1.0 { 4 } else { (slow * 3.0 + 1.0) as i32 };
    let palette: Vec<Block> = (0..variety)
        .map(|i| {
            let v = unit_noise(i.wrapping_mul(0x3_936D).wrapping_add(x) as f32 * 0.0005, z as f32 * 0.0005).clamp(0.0, 0.999_999_9);
            MEADOW[(v * 8.0) as usize]
        })
        .collect();
    let pick = if palette.len() == 1 {
        0
    } else {
        (unit_noise(x as f32 * 0.1, z as f32 * 0.1).clamp(0.0, 0.999_999_9) * palette.len() as f32) as usize
    };
    let block = palette[pick];
    if !can_survive(view, [x, y, z], block) || view.block(x, y, z) != Block::Air {
        return false;
    }
    if block == Block::TallGrass {
        view.set_block(x, y + 1, z, Block::TallGrassTop);
    }
    view.set_block(x, y, z, block)
}

/// Only when the position itself is air (then nothing is drawn).
pub fn flowers(view: &mut View, random: &mut Mt19937, at: [i32; 3], kind: FlowerKind, count: i32) -> bool {
    if view.block(at[0], at[1], at[2]) != Block::Air {
        return false;
    }
    let flower = pick(kind, at, random);
    let mut p = at;
    for _ in 0..count {
        if view.block(p[0], p[1], p[2]) == Block::Air && can_survive(view, p, flower) {
            view.set_block(p[0], p[1], p[2], flower);
        }
        p = zyx_offset(random, at);
    }
    true
}

/// In shade (149d9a560): under the light heightmap, or with a block that is not fully translucent
/// (`BlockType` +0x15c under 1.0) among the ten above; the heightmap is stale, so leaves a tree of
/// this chunk just grew count through the second test only.
fn shaded(view: &View, [x, y, z]: [i32; 3]) -> bool {
    view.heightmap(x, z) > y || (y + 1..=y + 10).any(|above| view.block(x, above, z).is_solid())
}

/// On mycelium, podzol or a log anywhere (oracle: a fallen log's mushroom stands in the open);
/// elsewhere only with the whole diamond around it shaded.
pub fn mushroom(view: &mut View, random: &mut Mt19937, at: [i32; 3], block: Block, count: i32) -> bool {
    let mut p = at;
    for _ in 0..count {
        let [x, y, z] = p;
        let below = view.block(x, y - 1, z);
        if view.block(x, y, z) == Block::Air && below != Block::Air {
            let soil = matches!(below, Block::Mycelium | Block::Podzol) || below.is_wood() && !below.is_leaves();
            if soil || SHADE.iter().all(|&[dx, dz]| shaded(view, [x + dx, y, z + dz])) && can_survive(view, p, block) {
                view.set_block(x, y, z, block);
            }
        }
        p = zyx_offset(random, at);
    }
    true
}
