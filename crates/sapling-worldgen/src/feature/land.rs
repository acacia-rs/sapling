//! The legacy land plants BDS builds in code (registered in 1482f4580): `DeadBushFeature::place`
//! (1482947e0), `ReedsFeature::place` (149d60080), `CactusFeature::place` (14827f3f0),
//! `MelonFeature::place` (149e9d840), `WaterlilyFeature::place` (14829d6c0) and
//! `BlockPileFeature::place` (148287420, pumpkins and sweet berries). Each try moves around the
//! feature position by `(r & m) - (r & m)` per axis, two raw draws each.

use super::place::Ctx;
use super::plants::can_survive;
use super::spec::Legacy;
use super::view::View;
use crate::block::Block;
use crate::random::Mt19937;
use crate::terrain::MIN_Y;

const PUMPKINS: [Block; 4] = [Block::PumpkinSouth, Block::PumpkinWest, Block::PumpkinNorth, Block::PumpkinEast];
const SWEET_BERRIES: [Block; 8] = [
    Block::SweetBerryBush0,
    Block::SweetBerryBush1,
    Block::SweetBerryBush2,
    Block::SweetBerryBush3,
    Block::SweetBerryBush4,
    Block::SweetBerryBush5,
    Block::SweetBerryBush6,
    Block::SweetBerryBush7,
];

/// `(r & mask) - (r & mask)`.
pub fn spread(random: &mut Mt19937, mask: u32) -> i32 {
    let a = (random.next_u32() & mask) as i32;
    a - (random.next_u32() & mask) as i32
}

/// The usual try offset: z ±7, y ±3, x ±7, drawn in that order.
pub fn zyx_offset(random: &mut Mt19937, [x, y, z]: [i32; 3]) -> [i32; 3] {
    let dz = spread(random, 7);
    let dy = spread(random, 3);
    let dx = spread(random, 7);
    [x + dx, y + dy, z + dz]
}

impl Legacy {
    pub fn place(&self, ctx: &mut Ctx, at: [i32; 3]) -> bool {
        match *self {
            Legacy::DeadBush => dead_bush(ctx.view, ctx.random, at),
            Legacy::Reeds => reeds(ctx.view, ctx.random, at),
            Legacy::Cactus => cactus(ctx.view, ctx.random, at),
            Legacy::Melon => melon(ctx.view, ctx.random, at),
            Legacy::Waterlily => waterlily(ctx.view, at),
            Legacy::Pumpkin { count } => pile(ctx.view, ctx.random, at, count, |random| PUMPKINS[random.next_int(4) as usize]),
            Legacy::SweetBerries { count } => pile(ctx.view, ctx.random, at, count, |random| SWEET_BERRIES[random.next_int(8) as usize]),
            Legacy::Mushroom { block, count } => super::flowers::mushroom(ctx.view, ctx.random, at, block, count),
            Legacy::Flower { kind, count } => super::flowers::flowers(ctx.view, ctx.random, at, kind, count),
            Legacy::Iceberg => super::iceberg::ice_feature(ctx.view, ctx.random, at),
            Legacy::Springs => super::springs::springs(ctx, at),
            Legacy::IcePatch => super::ice_spike::ice_patch(ctx.view, ctx.random, at),
            Legacy::IceSpike => super::ice_spike::ice_spike(ctx.view, ctx.random, at),
            Legacy::SeaPickle => super::aquatic::sea_pickle(ctx.view, ctx.random, at),
            Legacy::Bamboo => super::jungle::bamboo(ctx.view, ctx.random, at),
            Legacy::PodzolArea => super::jungle::podzol_area(ctx.view, ctx.random, at),
            Legacy::Vines => super::jungle::vines(ctx.view, ctx.random, at),
            Legacy::PaleMossVegetation => super::pale::vegetation(ctx.view, ctx.random, at),
            Legacy::Eyeblossom => super::pale::eyeblossom(ctx.view, at),
            Legacy::MeadowFlower => super::flowers::meadow(ctx.view, at),
            Legacy::GrassDisc => super::disk::grass_disc(ctx.view, ctx.random, at),
            Legacy::Pile(block) => super::pile::pile(ctx.view, ctx.random, at, block),
        }
    }
}

/// Down while the block above was air (the loop decrements after reading, so the base ends one
/// below the first non-air block), then four tries.
fn dead_bush(view: &mut View, random: &mut Mt19937, [x, mut y, z]: [i32; 3]) -> bool {
    let mut seen = None;
    while seen.is_none_or(|block| block == Block::Air) && y > MIN_Y {
        seen = Some(view.block(x, y, z));
        y -= 1;
    }
    for _ in 0..4 {
        let [px, py, pz] = zyx_offset(random, [x, y, z]);
        if view.block(px, py, pz) == Block::Air && can_survive(view, [px, py, pz], Block::Deadbush) {
            view.set_block(px, py, pz, Block::Deadbush);
        }
    }
    true
}

/// Twenty tries at the position's height, x then z drawn, each needing water beside the block below.
fn reeds(view: &mut View, random: &mut Mt19937, [x, y, z]: [i32; 3]) -> bool {
    for _ in 0..20 {
        let dx = spread(random, 7);
        let dz = spread(random, 7);
        let (px, pz) = (x + dx, z + dz);
        if view.block(px, y, pz) != Block::Air {
            continue;
        }
        let watered = [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().any(|&(ox, oz)| view.block(px + ox, y - 1, pz + oz) == Block::Water);
        if !watered {
            continue;
        }
        let bound = random.next_int(3) + 1;
        let height = random.next_int(bound) + 2;
        for i in 0..height {
            if view.block(px, y + i, pz) != Block::Air || !can_survive(view, [px, y + i, pz], Block::Reeds) {
                break;
            }
            view.set_block(px, y + i, pz, Block::Reeds);
        }
    }
    true
}

fn cactus(view: &mut View, random: &mut Mt19937, at: [i32; 3]) -> bool {
    for _ in 0..10 {
        let [px, py, pz] = zyx_offset(random, at);
        if view.block(px, py, pz) != Block::Air {
            continue;
        }
        let bound = random.next_int(3) + 1;
        let height = random.next_int(bound) + 1;
        for i in 0..height {
            if !can_survive(view, [px, py + i, pz], Block::Cactus) {
                break;
            }
            view.set_block(px, py + i, pz, Block::Cactus);
        }
    }
    true
}

fn melon(view: &mut View, random: &mut Mt19937, at: [i32; 3]) -> bool {
    for _ in 0..64 {
        let [px, py, pz] = zyx_offset(random, at);
        if view.block(px, py, pz) == Block::Air && view.block(px, py - 1, pz) == Block::GrassBlock {
            view.set_block(px, py, pz, Block::MelonBlock);
        }
    }
    true
}

/// No draws: on water, with nothing above it in the light heightmap.
fn waterlily(view: &mut View, [x, y, z]: [i32; 3]) -> bool {
    let placed = view.block(x, y, z) == Block::Air && view.block(x, y - 1, z) == Block::Water && view.heightmap(x, z) <= y;
    placed && view.set_block(x, y, z, Block::Waterlily)
}

/// On grass only; the block's state is drawn when it places. Offsets x, z, then y.
fn pile(view: &mut View, random: &mut Mt19937, [x, y, z]: [i32; 3], count: i32, block: impl Fn(&mut Mt19937) -> Block) -> bool {
    let mut p = [x, y, z];
    for _ in 0..count {
        let [px, py, pz] = p;
        if view.block(px, py, pz) == Block::Air && view.block(px, py - 1, pz) == Block::GrassBlock {
            let chosen = block(random);
            view.set_block(px, py, pz, chosen);
        }
        let dx = spread(random, 7);
        let dz = spread(random, 7);
        let dy = spread(random, 3);
        p = [x + dx, y + dy, z + dz];
    }
    true
}
