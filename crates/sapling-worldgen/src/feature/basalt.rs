//! The basalt features BDS builds in code (registered in 1482f4580): `BasaltColumnsFeature::place` (14827c230),
//! `BasaltPillarFeature::place` (14827ca10) and `DeltaFeature::place` (148294fc0). Tests read the cell's
//! material: `solid` is `Material::mSolid`, `air` the air material.

use super::view::View;
use crate::block::Block;
use crate::random::Mt19937;

/// The Nether's `BlockSource` height range: `getMinHeight` (slot 0x118), `getMaxHeight` (slot 0x120).
pub const BOTTOM: i32 = 0;
pub const TOP: i32 = crate::nether::HEIGHT;
/// x−1, x+1, z−1, z+1.
const SIDES: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

/// `Material::mSolid`: not air, a liquid, a plant or fire.
pub fn solid(block: Block) -> bool {
    block.is_solid()
}

pub fn columns(view: &mut View, random: &mut Mt19937, [x, mut y, z]: [i32; 3]) -> bool {
    let surface = loop {
        if y <= BOTTOM || y >= TOP {
            return false;
        }
        if !solid(view.block(x, y, z)) && view.block(x, y - 1, z).is_opaque_solid() {
            break y;
        }
        y -= 1;
    };
    let big = random.next_float() >= 0.3;
    let reach = i32::from(big) + 1;
    let height = if big { random.next_int(6) + 5 } else { random.next_int(4) + 1 };
    let (tries, range, offset) = if random.next_float() < 0.9 { (50, 11, -5) } else { (15, 17, -8) };
    for _ in 0..tries {
        let px = x + offset + random.next_int(range);
        let pz = z + offset + random.next_int(range);
        let left = height - (px - x).abs() - (pz - z).abs();
        if left <= 0 {
            continue;
        }
        let r = random.next_int(reach) + reach;
        'columns: for cx in px - r..=px + r {
            for cz in pz - r..=pz + r {
                let steps = (cx - px).abs() + (cz - pz).abs();
                if let Some(base) = column_base(view, [cx, surface, cz], steps) {
                    grow_column(view, [cx, base, cz], left - steps / 2);
                    break 'columns;
                }
            }
        }
    }
    true
}

/// Up from a solid cell, else down, at most `steps` cells: the first open cell over an opaque solid block,
/// if that block is plain basalt or blackstone (else the column is given up).
fn column_base(view: &View, [x, mut y, z]: [i32; 3], mut steps: i32) -> Option<i32> {
    let step = if solid(view.block(x, y, z)) { 1 } else { -1 };
    while y > BOTTOM {
        if y >= TOP || steps < 1 {
            return None;
        }
        let below = view.block(x, y - 1, z);
        if !solid(view.block(x, y, z)) && below.is_opaque_solid() {
            return matches!(below, Block::Basalt | Block::Blackstone).then_some(y);
        }
        y += step;
        steps -= 1;
    }
    None
}

/// `count + 1` basalt up from the base, over anything but bedrock.
fn grow_column(view: &mut View, [x, mut y, z]: [i32; 3], mut count: i32) {
    while view.block(x, y, z) != Block::Bedrock && y < TOP && count >= 0 {
        view.set_block(x, y, z, Block::Basalt);
        y += 1;
        count -= 1;
    }
}

pub fn pillar(view: &mut View, random: &mut Mt19937, [x, mut y, z]: [i32; 3]) -> bool {
    if view.block(x, y, z) != Block::Air || !matches!(view.block(x, y + 1, z), Block::SoulSand | Block::SoulSoil | Block::Netherrack) {
        return false;
    }
    let (scale, base) = (f32::from_bits(0x3da3_d708), f32::from_bits(0x3f5e_b852));
    let chances: [f32; 4] = std::array::from_fn(|_| random.next_float() * scale + base);
    loop {
        // Compared with 100: always passes.
        random.next_float();
        if view.block(x, y, z) != Block::Air || view.block(x, y + 1, z) == Block::Air {
            break;
        }
        view.set_block(x, y, z, Block::Basalt);
        for (&(dx, dz), &chance) in SIDES.iter().zip(&chances) {
            let roll = random.next_float();
            if roll < chance && view.block(x + dx, y, z + dz) == Block::Air && view.block(x + dx, y + 1, z + dz) != Block::Air {
                view.set_block(x + dx, y, z + dz, Block::Basalt);
            }
        }
        y -= 1;
    }
    for (dx, dz) in SIDES {
        if view.block(x + dx, y, z + dz) != Block::Air {
            view.set_block(x + dx, y + 1, z + dz, Block::Basalt);
        }
    }
    for dx in -3i32..=3 {
        for dz in -3i32..=3 {
            if random.next_int(10) < (dx * dz).abs() {
                continue;
            }
            let (qx, qz) = (x + dx, z + dz);
            if let Some(drop) = (0..3).find(|drop| view.block(qx, y - 1 - drop, qz) != Block::Air) {
                view.set_block(qx, y - drop, qz, Block::Basalt);
            }
        }
    }
    true
}

pub fn delta(view: &mut View, random: &mut Mt19937, [x, mut y, z]: [i32; 3]) -> bool {
    let step = if solid(view.block(x, y, z)) { 1 } else { -1 };
    loop {
        if y < 2 || y >= TOP {
            return false;
        }
        let below = view.block(x, y - 1, z);
        if view.block(x, y, z) == Block::Air && below != Block::Bedrock && below != Block::Air && below != Block::Lava {
            break;
        }
        y += step;
    }
    let rim = random.next_float() < 0.9;
    let (ox, oz) = if rim { (random.next_int(3), random.next_int(3)) } else { (0, 0) };
    let rx = random.next_int(4) + 3;
    let rz = random.next_int(4) + 3;
    let reach = rx.max(rz);
    let rim = rim && ox != 0 && oz != 0;
    for dx in -rx..=rx {
        for dz in -rz..=rz {
            if dx.abs() + dz.abs() > reach {
                continue;
            }
            let c = [x + dx, y, z + dz];
            if !delta_cell(view, c) {
                continue;
            }
            if rim {
                view.set_block(c[0], c[1], c[2], Block::Magma);
            }
            let [lx, ly, lz] = [c[0] + ox, y, c[2] + oz];
            if delta_cell(view, [lx, ly, lz]) {
                view.set_block(lx, ly, lz, Block::Lava);
            }
        }
    }
    true
}

/// `DeltaFeature::_isValidPlacement` (1482958f0): not lava, open above, closed below and on the four sides.
fn delta_cell(view: &View, [x, y, z]: [i32; 3]) -> bool {
    let closed = [(0, -1, 0), (0, 0, -1), (0, 0, 1), (1, 0, 0), (-1, 0, 0)];
    view.block(x, y, z).name() != "lava" && view.block(x, y + 1, z) == Block::Air && closed.iter().all(|&(dx, dy, dz)| view.block(x + dx, y + dy, z + dz) != Block::Air)
}
