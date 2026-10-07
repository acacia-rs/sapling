//! The tree, foliage and rock features BDS builds in code (registered in 1482f4580):
//! `LegacyTreeFeature` (ctor 149e981a0, place 149e9cf40, tree loop 149e97fc0, pickers in its
//! constructor's lambdas), `LegacyForestFoliageFeature` (149e95c70) with `DoublePlantFeature`
//! (148299900), `LegacySwampFoliageFeature` (149e97c00), `LegacyForestRockFeature` (149e96360)
//! with the mossy cobblestone blob (14827dc10) and `LeafLitterFeature` (149e82710).

use super::data::FEATURES;
use super::land::zyx_offset;
use super::place::Ctx;
use super::plants::may_place;
use super::spec::{LegacyTree, TreeChoice};
use super::view::View;
use crate::block::Block;
use crate::random::Mt19937;
use crate::terrain::MIN_Y;

const DOUBLE_PLANTS: [(Block, Block); 3] = [(Block::Lilac, Block::LilacTop), (Block::RoseBush, Block::RoseBushTop), (Block::Peony, Block::PeonyTop)];
const ROCK_GROUND: [Block; 4] = [Block::GrassBlock, Block::Stone, Block::Dirt, Block::CoarseDirt];
/// `cardinal_direction` 0..4 (south, west, north, east) by `growth` 0..4.
const LEAF_LITTER: [[Block; 4]; 4] = [
    [Block::LeafLitter0South, Block::LeafLitter0West, Block::LeafLitter0North, Block::LeafLitter0East],
    [Block::LeafLitter1South, Block::LeafLitter1West, Block::LeafLitter1North, Block::LeafLitter1East],
    [Block::LeafLitter2South, Block::LeafLitter2West, Block::LeafLitter2North, Block::LeafLitter2East],
    [Block::LeafLitter3South, Block::LeafLitter3West, Block::LeafLitter3North, Block::LeafLitter3East],
];
const PINK_PETALS: [[Block; 4]; 4] = [
    [Block::PinkPetals0South, Block::PinkPetals0West, Block::PinkPetals0North, Block::PinkPetals0East],
    [Block::PinkPetals1South, Block::PinkPetals1West, Block::PinkPetals1North, Block::PinkPetals1East],
    [Block::PinkPetals2South, Block::PinkPetals2West, Block::PinkPetals2North, Block::PinkPetals2East],
    [Block::PinkPetals3South, Block::PinkPetals3West, Block::PinkPetals3North, Block::PinkPetals3East],
];

impl LegacyTree {
    pub fn place(&self, ctx: &mut Ctx, at: [i32; 3]) -> Option<[i32; 3]> {
        match *self {
            LegacyTree::Trees { count, choice, features, grass } => {
                let count = tree_count(ctx.random, count);
                trees(ctx, at, count, grass, |random| features[choice.pick(random)]);
            }
            LegacyTree::ForestFoliage { flower_forest } => forest_foliage(ctx.view, ctx.random, at, flower_forest),
            LegacyTree::SwampFoliage { seagrass, tree, huge_mushroom, grass } => {
                let count = if ctx.random.next_int(10) == 0 {
                    let f = ctx.random.next_float();
                    (f + f) as i32
                } else {
                    ctx.random.next_int(10).max(7) - 5
                };
                for _ in 0..10 {
                    let dx = ctx.random.next_int(16);
                    let dz = ctx.random.next_int(16);
                    FEATURES[seagrass].place(ctx, [at[0] + dx, at[1] + 54, at[2] + dz]);
                }
                trees(ctx, at, count, grass, |random| if random.next_int(20) == 0 { huge_mushroom } else { tree });
            }
            LegacyTree::ForestRock => {
                for _ in 0..ctx.random.next_int(3) {
                    let dz = ctx.random.next_int(16);
                    let dx = ctx.random.next_int(16);
                    let (x, z) = (at[0] + dx, at[2] + dz);
                    rock(ctx.view, ctx.random, [x, ctx.view.heightmap(x, z), z]);
                }
            }
            LegacyTree::HugeMushroom => return super::huge_mushroom::place(ctx.view, ctx.random, at).then_some(at),
            LegacyTree::FlowerBed { petals } => {
                let growth = ctx.random.next_int(4) as usize;
                let facing = ctx.random.next_int(4) as usize;
                let [x, y, z] = at;
                let below = ctx.view.block(x, y - 1, z);
                let (block, supported) = if petals {
                    (PINK_PETALS[growth][facing], super::plants::is_soil(below))
                } else {
                    (LEAF_LITTER[growth][facing], below.blocks_motion())
                };
                let placed = ctx.view.block(x, y, z) == Block::Air && supported;
                return (placed && ctx.view.set_block(x, y, z, block)).then_some(at);
            }
        }
        Some(at)
    }
}

/// One draw of ten picks the formula: `count * nextFloat`, or `count + max(nextInt(10), 7) - 7`,
/// or (for counts under one) a single tree with chance `count`.
fn tree_count(random: &mut Mt19937, count: f32) -> i32 {
    if random.next_int(10) == 0 {
        (count * random.next_float()) as i32
    } else if count >= 1.0 {
        (count + random.next_int(10).max(7) as f32) as i32 - 7
    } else {
        i32::from(random.next_float() < count)
    }
}

/// 149e97fc0: per tree the picker, then z and x `nextInt(14) + 1` into the chunk and the column's
/// heightmap; the grass feature follows at the same place (BDS skips it on legacy worlds only).
fn trees(ctx: &mut Ctx, at: [i32; 3], count: i32, grass: usize, mut pick: impl FnMut(&mut Mt19937) -> usize) {
    for _ in 0..count {
        let feature = pick(ctx.random);
        let z = at[2] + ctx.random.next_int(14) + 1;
        let x = at[0] + ctx.random.next_int(14) + 1;
        let p = [x, ctx.view.heightmap(x, z), z];
        FEATURES[feature].place(ctx, p);
        FEATURES[grass].place(ctx, p);
    }
}

fn one_in(random: &mut Mt19937, n: i32) -> bool {
    random.next_int(n) == 0
}

impl TreeChoice {
    fn pick(self, random: &mut Mt19937) -> usize {
        match self {
            TreeChoice::First => 0,
            TreeChoice::OneInThree => usize::from(!one_in(random, 3)),
            TreeChoice::FourInFive => usize::from(random.next_int(5) <= 0),
            TreeChoice::BirchMutated => {
                if random.next_int(5) == 0 {
                    0
                } else {
                    usize::from(random.next_boolean())
                }
            }
            TreeChoice::ExtremeHills => {
                if random.next_int(3) > 0 {
                    0
                } else if random.next_int(10) == 0 {
                    1
                } else {
                    2
                }
            }
            TreeChoice::Forest => {
                if one_in(random, 5) {
                    0
                } else if one_in(random, 10) {
                    1
                } else {
                    2
                }
            }
            TreeChoice::Meadow => usize::from(!random.next_boolean()),
            TreeChoice::Jungle => {
                if one_in(random, 10) {
                    0
                } else if one_in(random, 2) {
                    1
                } else if one_in(random, 3) {
                    2
                } else {
                    3
                }
            }
            TreeChoice::JungleEdge => {
                if one_in(random, 10) {
                    0
                } else if one_in(random, 2) {
                    1
                } else {
                    2
                }
            }
            TreeChoice::BambooJungle => {
                if random.next_int(20) == 0 {
                    0
                } else if random.next_int(20) < 3 {
                    1
                } else if random.next_int(10) < 7 {
                    2
                } else {
                    3
                }
            }
            TreeChoice::MegaTaiga => {
                if one_in(random, 3) {
                    usize::from(!one_in(random, 13))
                } else if one_in(random, 3) {
                    2
                } else {
                    3
                }
            }
            TreeChoice::RedwoodMutated => {
                if one_in(random, 3) {
                    0
                } else if one_in(random, 3) {
                    1
                } else {
                    2
                }
            }
        }
    }
}

/// Lilacs, rose bushes or peonies: `nextInt(5)` less 1 (flower forests) or 3 patches, each of five
/// tries at a random height under the column's heightmap plus 32.
fn forest_foliage(view: &mut View, random: &mut Mt19937, at: [i32; 3], flower_forest: bool) {
    let count = random.next_int(5) - if flower_forest { 1 } else { 3 };
    for _ in 0..count {
        let plant = DOUBLE_PLANTS[random.next_int(3) as usize];
        for _ in 0..5 {
            let dx = random.next_int(16);
            let dz = random.next_int(16);
            let (x, z) = (at[0] + dx, at[2] + dz);
            // The bound is the absolute heightmap y plus 32, so heights run from 0, not the world bottom.
            let height = view.heightmap(x, z);
            if height + 32 > 0 {
                let y = random.next_int(height + 32);
                double_plant(view, random, [x, y, z], plant);
            }
        }
    }
}

/// `DoublePlantFeature::place` (148299900): nothing (and no draws) when every chunk within 7
/// blocks has its highest block below `y - 4`, else 64 tries around the position.
fn double_plant(view: &mut View, random: &mut Mt19937, [x, y, z]: [i32; 3], (lower, upper): (Block, Block)) {
    let highest = [(-7, -7), (7, -7), (-7, 7), (7, 7)].iter().filter_map(|&(dx, dz)| view.highest_block(x + dx, z + dz)).max();
    if highest.is_some_and(|top| top < y - 4) {
        return;
    }
    for _ in 0..64 {
        let [px, py, pz] = zyx_offset(random, [x, y, z]);
        if view.block(px, py, pz) == Block::Air && may_place(view, [px, py, pz], lower) {
            view.set_block(px, py, pz, lower);
            view.set_block(px, py + 1, pz, upper);
        }
    }
}

/// The mossy cobblestone blob: down to grass, stone or dirt, then three blobs of radius 0..1 per
/// axis, each moved by up to one block (x, down, z).
fn rock(view: &mut View, random: &mut Mt19937, [mut x, mut y, mut z]: [i32; 3]) {
    while y > MIN_Y + 3 {
        let below = view.block(x, y - 1, z);
        if below != Block::Air && ROCK_GROUND.contains(&below) {
            break;
        }
        y -= 1;
    }
    if y <= MIN_Y + 3 {
        return;
    }
    for _ in 0..3 {
        let a = random.next_int(2);
        let b = random.next_int(2);
        let c = random.next_int(2);
        let radius = (a + b + c) as f32 * 0.333 + 0.5;
        for cx in x - a..=x + a {
            for cy in y - b..=y + b {
                for cz in z - c..=z + c {
                    let d = (cx - x) * (cx - x) + (cy - y) * (cy - y) + (cz - z) * (cz - z);
                    if d as f32 <= radius * radius {
                        view.set_block(cx, cy, cz, Block::MossyCobblestone);
                    }
                }
            }
        }
        x += random.next_int(2) - 1;
        y -= random.next_int(2);
        z += random.next_int(2) - 1;
    }
}
