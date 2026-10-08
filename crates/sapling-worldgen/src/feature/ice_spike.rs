//! The ice plains spikes' hardcoded features: `IcePatchFeature::place` (149e8de60) and
//! `IceSpikeFeature::place` (149e8e4e0), Java 1.12's ice patch and ice spike with packed ice also
//! taking grass blocks and dirt (the dirt list 14ccdb858: dirt, coarse dirt).

use super::view::View;
use crate::block::Block;
use crate::random::Mt19937;
use crate::terrain::MIN_Y;

/// The ice patch's `+0x28`.
const PATCH_RADIUS: i32 = 4;

/// Down through air to the ground, never below the world's floor plus 2.
fn ground(view: &View, [x, mut y, z]: [i32; 3]) -> [i32; 3] {
    while view.block(x, y, z) == Block::Air && y > MIN_Y + 2 {
        y -= 1;
    }
    [x, y, z]
}

fn dirt_snow_or_ice(block: Block) -> bool {
    matches!(block, Block::Dirt | Block::CoarseDirt | Block::Snow | Block::Ice)
}

/// A disc of packed ice over dirt, snow and ice, 2 or 3 blocks across the snow it stands on.
pub fn ice_patch(view: &mut View, random: &mut Mt19937, at: [i32; 3]) -> bool {
    let [x, y, z] = ground(view, at);
    if view.block(x, y, z) != Block::Snow {
        return false;
    }
    let r = random.next_int(PATCH_RADIUS - 2) + 2;
    for px in x - r..=x + r {
        for pz in z - r..=z + r {
            if (px - x) * (px - x) + (pz - z) * (pz - z) > r * r {
                continue;
            }
            for py in y - 1..=y + 1 {
                if dirt_snow_or_ice(view.block(px, py, pz)) {
                    view.set_block(px, py, pz, Block::PackedIce);
                }
            }
        }
    }
    true
}

/// Packed ice over air, grass, dirt, snow and ice.
fn icy(block: Block) -> bool {
    block == Block::Air || block == Block::GrassBlock || dirt_snow_or_ice(block)
}

/// A packed ice spike on snow, mirrored below for its taller rings, and a root column down to y 50.
pub fn ice_spike(view: &mut View, random: &mut Mt19937, at: [i32; 3]) -> bool {
    let [x, mut y, z] = ground(view, at);
    if view.block(x, y, z) != Block::Snow {
        return false;
    }
    y += random.next_int(4);
    let height = random.next_int(4) + 7;
    let width = height / 4 + random.next_int(2);
    if width > 1 && random.next_int(60) == 0 {
        y += 10 + random.next_int(30);
    }
    for k in 0..height {
        let f = (1.0 - k as f32 / height as f32) * width as f32;
        let l = f.ceil() as i32;
        for dx in -l..=l {
            let fx = dx.abs() as f32 - 0.25;
            for dz in -l..=l {
                let fz = dz.abs() as f32 - 0.25;
                let inside = (dx == 0 && dz == 0) || fx * fx + fz * fz <= f * f;
                let edge = dx == -l || dx == l || dz == -l || dz == l;
                if !inside || edge && random.next_float() > 0.75 {
                    continue;
                }
                if icy(view.block(x + dx, y + k, z + dz)) {
                    view.set_block(x + dx, y + k, z + dz, Block::PackedIce);
                }
                if k != 0 && l > 1 && icy(view.block(x + dx, y - k, z + dz)) {
                    view.set_block(x + dx, y - k, z + dz, Block::PackedIce);
                }
            }
        }
    }
    let root = (width - 1).clamp(0, 1);
    for dx in -root..=root {
        for dz in -root..=root {
            let mut py = y - 1;
            let mut left = if dx.abs() == 1 && dz.abs() == 1 { random.next_int(5) } else { 50 };
            while py > 50 {
                let block = view.block(x + dx, py, z + dz);
                if !(icy(block) || block == Block::PackedIce) {
                    break;
                }
                view.set_block(x + dx, py, z + dz, Block::PackedIce);
                py -= 1;
                if left < 2 {
                    py -= random.next_int(5) + 1;
                    left = random.next_int(5);
                } else {
                    left -= 1;
                }
            }
        }
    }
    true
}
