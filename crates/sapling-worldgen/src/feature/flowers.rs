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

/// The diamond of radius 2 the mushroom checks for shade, in BDS's order.
const SHADE: [[i32; 2]; 13] =
    [[0, 0], [0, -1], [1, 0], [0, 1], [-1, 0], [1, -1], [-1, -1], [1, 1], [-1, 1], [0, -2], [2, 0], [0, 2], [-2, 0]];

/// The flower of one call: nothing is drawn for swamps, one or two draws otherwise.
fn pick(kind: FlowerKind, [x, _, z]: [i32; 3], random: &mut Mt19937) -> Block {
    match kind {
        FlowerKind::Swamp => Block::BlueOrchid,
        FlowerKind::Forest => {
            if random.next_boolean() {
                Block::LilyOfTheValley
            } else {
                Block::Dandelion
            }
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
        FlowerKind::Overworld | FlowerKind::FlowerForest => {
            if random.next_int(3) == 0 {
                Block::Dandelion
            } else {
                Block::Poppy
            }
        }
    }
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

/// In shade (149d9a560): under the light heightmap. BDS also counts partly see-through blocks within
/// ten above, which only trees' leaves would be.
fn shaded(view: &View, [x, y, z]: [i32; 3]) -> bool {
    view.heightmap(x, z) > y
}

/// On mycelium or podzol anywhere; elsewhere only with the whole diamond around it shaded.
pub fn mushroom(view: &mut View, random: &mut Mt19937, at: [i32; 3], block: Block, count: i32) -> bool {
    let mut p = at;
    for _ in 0..count {
        let [x, y, z] = p;
        let below = view.block(x, y - 1, z);
        if view.block(x, y, z) == Block::Air && below != Block::Air {
            let soil = matches!(below, Block::Mycelium | Block::Podzol);
            if soil || SHADE.iter().all(|&[dx, dz]| shaded(view, [x + dx, y, z + dz])) && can_survive(view, p, block) {
                view.set_block(x, y, z, block);
            }
        }
        p = zyx_offset(random, at);
    }
    true
}
