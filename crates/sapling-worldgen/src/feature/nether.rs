//! The Nether's hardcoded decoration (`NetherGenerator::decorationPostProcessChunk` 1481ed1f0): Java 1.12's hell
//! features on the chunk's post-process generator, the biome rules on it reseeded, then ancient debris
//! (docs/research/worldgen-bds-nether.md, "Decoration"). Every position draws x, then y, then z.

use super::flowers::mushroom;
use super::land::{spread, zyx_offset};
use super::ore::exposed;
use super::spec::{Ore, ReplaceRule};
use super::view::View;
use crate::block::Block;
use crate::random::Mt19937;

static GOLD: [ReplaceRule; 1] = [ReplaceRule { places: Block::NetherGoldOre, may_replace: &[Block::Netherrack] }];
static QUARTZ: [ReplaceRule; 1] = [ReplaceRule { places: Block::QuartzOre, may_replace: &[Block::Netherrack] }];
static MAGMA: [ReplaceRule; 1] = [ReplaceRule { places: Block::Magma, may_replace: &[Block::Netherrack, Block::Blackstone] }];
static GRAVEL: [ReplaceRule; 1] = [ReplaceRule { places: Block::Gravel, may_replace: &[Block::Netherrack] }];
static BLACKSTONE: [ReplaceRule; 1] = [ReplaceRule { places: Block::Blackstone, may_replace: &[Block::Netherrack] }];

/// The faces of `nextInt(6)` (14a712760).
const STEPS: [[i32; 3]; 6] = [[0, -1, 0], [0, 1, 0], [0, 0, -1], [0, 0, 1], [-1, 0, 0], [1, 0, 0]];

/// x from `x0`, y from `y` (lowest, span), z from `z0`.
fn position(random: &mut Mt19937, (x0, z0): (i32, i32), (low, span): (i32, i32)) -> [i32; 3] {
    let x = x0 + random.next_int(16);
    let y = random.next_int(span) + low;
    [x, y, z0 + random.next_int(16)]
}

/// Steps 1 to 12, before the biome rules.
pub fn before_rules(random: &mut Mt19937, chunk: (i32, i32), view: &mut View) {
    let centre = (chunk.0 * 16 + 8, chunk.1 * 16 + 8);
    let corner = (chunk.0 * 16, chunk.1 * 16);
    for _ in 0..8 {
        spring(view, position(random, centre, (4, 120)), false);
    }
    let fires = {
        let bound = random.next_int(10) + 1;
        random.next_int(bound) + 1
    };
    for _ in 0..fires {
        let at = position(random, centre, (4, 120));
        fire(view, random, at);
    }
    let glows = {
        let bound = random.next_int(10) + 1;
        random.next_int(bound)
    };
    for _ in 0..glows {
        let at = position(random, centre, (4, 120));
        glowstone(view, random, at);
    }
    for _ in 0..10 {
        let at = position(random, centre, (0, 128));
        glowstone(view, random, at);
    }
    for block in [Block::BrownMushroom, Block::RedMushroom] {
        if random.next_int(1) == 0 {
            let at = position(random, centre, (0, 128));
            mushroom(view, random, at, block, 64);
        }
    }
    let ores: [(&'static [ReplaceRule], i32, i32, (i32, i32)); 2] = [(&GOLD, 10, 10, (10, 108)), (&QUARTZ, 14, 16, (10, 108))];
    for (rules, size, count, range) in ores {
        place_ores(view, random, corner, Ore { count: size, discard_on_air: 0.0, rules }, count, range);
    }
    for _ in 0..16 {
        spring(view, position(random, corner, (10, 108)), true);
    }
    place_ores(view, random, corner, Ore { count: 28, discard_on_air: 0.0, rules: &MAGMA }, 9, (23, 14));
    place_ores(view, random, corner, Ore { count: 33, discard_on_air: 0.0, rules: &GRAVEL }, 2, (5, 32));
    place_ores(view, random, corner, Ore { count: 33, discard_on_air: 0.0, rules: &BLACKSTONE }, 2, (5, 32));
}

/// Steps 15 and 16, on the generator the rules' `math.random` left.
pub fn after_rules(random: &mut Mt19937, chunk: (i32, i32), view: &mut View) {
    let corner = (chunk.0 * 16, chunk.1 * 16);
    for (count, max, range) in [(2, 4, (8, 16)), (3, 3, (8, 112))] {
        for _ in 0..count {
            let at = position(random, corner, range);
            debris(view, random, at, max);
        }
    }
}

fn place_ores(view: &mut View, random: &mut Mt19937, corner: (i32, i32), ore: Ore, count: i32, range: (i32, i32)) {
    for _ in 0..count {
        let at = position(random, corner, range);
        ore.place(view, at, random);
    }
}

/// `HellSpringFeature` (149d5c8c0): a lava source in netherrack, open to air on one side unless `inside_rock`.
/// Its caller queues a tick in 0 at the cell whether or not it placed.
fn spring(view: &mut View, [x, y, z]: [i32; 3], inside_rock: bool) {
    let there = view.block(x, y, z);
    if view.block(x, y + 1, z) == Block::Netherrack && (there == Block::Air || there == Block::Netherrack) {
        let sides = [[x - 1, y, z], [x + 1, y, z], [x, y, z - 1], [x, y, z + 1], [x, y - 1, z]];
        let count = |block: Block| sides.iter().filter(|&&[x, y, z]| view.block(x, y, z) == block).count();
        let (rock, air) = (count(Block::Netherrack), count(Block::Air));
        if rock == 4 && !inside_rock && air == 1 || rock == 5 {
            view.set_block(x, y, z, Block::Lava);
        }
    }
    view.schedule([x, y, z], true, 0);
}

/// `HellFireFeature` (149d5b460): 64 tries, each offset drawn before its check.
fn fire(view: &mut View, random: &mut Mt19937, at: [i32; 3]) {
    for _ in 0..64 {
        let [x, y, z] = zyx_offset(random, at);
        if view.block(x, y, z) != Block::Air {
            continue;
        }
        match view.block(x, y - 1, z) {
            Block::Netherrack => view.set_block(x, y, z, Block::Fire),
            Block::SoulSoil => view.set_block(x, y, z, Block::SoulFire),
            _ => false,
        };
    }
}

/// `GlowStoneFeature` (149e8c580): a seed under the roof, then 1500 tries growing off exactly one glowstone face.
fn glowstone(view: &mut View, random: &mut Mt19937, [x, y, z]: [i32; 3]) {
    if view.block(x, y, z) != Block::Air || !matches!(view.block(x, y + 1, z), Block::Netherrack | Block::Basalt | Block::Blackstone) {
        return;
    }
    view.set_block(x, y, z, Block::Glowstone);
    let faces = [[0, -1, 0], [0, 1, 0], [-1, 0, 0], [1, 0, 0], [0, 0, -1], [0, 0, 1]];
    for _ in 0..1500 {
        let dz = spread(random, 7);
        let dy = random.next_int(12);
        let dx = spread(random, 7);
        let [tx, ty, tz] = [x + dx, y - dy, z + dz];
        if view.block(tx, ty, tz) != Block::Air {
            continue;
        }
        let touching = faces.iter().filter(|[fx, fy, fz]| view.block(tx + fx, ty + fy, tz + fz) == Block::Glowstone).count();
        if touching == 1 {
            view.set_block(tx, ty, tz, Block::Glowstone);
        }
    }
}

/// `AncientDebrisFeature` (149d5cd10): `nextInt(max)` walks of 2i face steps from the origin, debris where a
/// walk ends in netherrack within 6 blocks and out of the air.
fn debris(view: &mut View, random: &mut Mt19937, origin: [i32; 3], max: i32) {
    let count = random.next_int(max);
    for i in 0..count {
        let mut p = origin;
        for _ in 0..2 * i {
            let [sx, sy, sz] = STEPS[random.next_int(6) as usize];
            p = [p[0] + sx, p[1] + sy, p[2] + sz];
        }
        let [dx, dy, dz] = [p[0] - origin[0], p[1] - origin[1], p[2] - origin[2]];
        if dx * dx + dy * dy + dz * dz > 36 {
            continue;
        }
        if view.block(p[0], p[1], p[2]) == Block::Netherrack && !exposed(view, p) {
            view.set_block(p[0], p[1], p[2], Block::AncientDebris);
        }
    }
}
