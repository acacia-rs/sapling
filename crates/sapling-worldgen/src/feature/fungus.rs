//! `HugeFungusFeature::place` (143b7ded0), one class for both forests: a stem, a hat of wart with
//! shroomlights, and on the crimson one weeping vines under the hat.

use super::basalt::{BOTTOM, TOP};
use super::nether_vines::{vine_length, weeping_column};
use super::plants::is_nether_soil;
use super::view::View;
use crate::block::Block;
use crate::random::Mt19937;

struct Fungus {
    stem: Block,
    hat: Block,
    vines: bool,
}

/// `BlockType::canBeBuiltOver` (vtable 0x298) for what grows in the Nether. Not vines (oracle: a hat leaves
/// another cluster's twisting vines and draws nothing for their cells).
fn buildable(block: Block) -> bool {
    matches!(block, Block::Air | Block::CrimsonRoots | Block::WarpedRoots | Block::NetherSprouts | Block::Fire | Block::SoulFire) || block.is_fluid()
}

pub fn huge_fungus(view: &mut View, random: &mut Mt19937, [x, y, z]: [i32; 3], crimson: bool) -> bool {
    let fungus = if crimson {
        Fungus { stem: Block::CrimsonStem, hat: Block::NetherWartBlock, vines: true }
    } else {
        Fungus { stem: Block::WarpedStem, hat: Block::WarpedWartBlock, vines: false }
    };
    let height = (random.next_int(10) + 4) << u32::from(random.next_int(12) == 0);
    if y <= BOTTOM || y + height + 1 >= TOP {
        return false;
    }
    let Some(y) = base(view, [x, y, z]) else { return false };
    let roll = random.next_float();
    let huge = (roll as f64) < 0.06;
    stem(view, random, [x, y, z], &fungus, height, huge);
    hat(view, random, [x, y, z], &fungus, height, huge);
    true
}

/// Down at most 100 cells to the first one standing on nylium, soul soil or soil; lava fails.
fn base(view: &View, [x, mut y, z]: [i32; 3]) -> Option<i32> {
    for _ in 0..=100 {
        let below = view.block(x, y - 1, z);
        if view.block(x, y, z).is_lava() || below.is_lava() {
            return None;
        }
        if is_nether_soil(below) {
            return Some(y);
        }
        y -= 1;
        if y <= BOTTOM {
            return None;
        }
    }
    None
}

fn stem(view: &mut View, random: &mut Mt19937, [x, y, z]: [i32; 3], fungus: &Fungus, height: i32, huge: bool) {
    let r = i32::from(huge);
    for dx in -r..=r {
        for dz in -r..=r {
            let corner = huge && dx.abs() == 1 && dz.abs() == 1;
            for dy in 0..height.max(1) {
                let [cx, cy, cz] = [x + dx, y + dy, z + dz];
                let there = view.block(cx, cy, cz);
                if !(buildable(there) || matches!(there, Block::CrimsonFungus | Block::WarpedFungus)) {
                    continue;
                }
                if !corner || random.next_float() < 0.1 {
                    view.set_block(cx, cy, cz, fungus.stem);
                }
            }
        }
    }
}

fn hat(view: &mut View, random: &mut Mt19937, [x, y, z]: [i32; 3], fungus: &Fungus, height: i32, huge: bool) {
    let size = (random.next_int(height / 3 + 1) + 5).min(height);
    let low = height - size;
    for k in low..=height {
        let r = random.next_int(3);
        let radius = if size >= 9 && k < low + 4 { 3 } else if k < height - r { 2 } else { 1 };
        let edge = radius + i32::from(huge);
        for dx in -edge..=edge {
            for dz in -edge..=edge {
                let at = [x + dx, y + k, z + dz];
                let there = view.block(at[0], at[1], at[2]);
                if there.is_opaque_solid() || !buildable(there) {
                    continue;
                }
                let (on_x, on_z, top) = (dx.abs() == edge, dz.abs() == edge, k == height);
                if k < low + 3 {
                    if on_x || on_z || top {
                        drop_cell(view, random, at, fungus);
                    }
                } else {
                    upper_cell(view, random, at, fungus, (on_x || top || on_z, on_x && on_z));
                }
            }
        }
    }
}

/// A cell of the hat's lowest layers, on its rim: hat over hat, else a chance of hat with vines.
fn drop_cell(view: &mut View, random: &mut Mt19937, [x, y, z]: [i32; 3], fungus: &Fungus) {
    if view.block(x, y - 1, z) == fungus.hat {
        view.set_block(x, y, z, fungus.hat);
    } else if random.next_float() < 0.15 {
        hat_with_vines(view, random, [x, y, z], fungus, 11);
    }
}

/// `(rim, corner)`: the rim is the hat's sides and its top layer.
fn upper_cell(view: &mut View, random: &mut Mt19937, [x, y, z]: [i32; 3], fungus: &Fungus, (rim, corner): (bool, bool)) {
    let roll = random.next_float();
    if !rim {
        if roll < 0.1 {
            view.set_block(x, y, z, Block::Shroomlight);
        } else if random.next_float() < 0.2 {
            hat_with_vines(view, random, [x, y, z], fungus, 10);
        }
        return;
    }
    let (hat_below, chance) = if corner { (0.7, 12) } else { (0.98, 14) };
    if roll < hat_below {
        hat_with_vines(view, random, [x, y, z], fungus, chance);
    } else if random.next_float() < 0.02 {
        view.set_block(x, y, z, Block::Shroomlight);
    }
}

/// Hat, then vines below it in one of `chance` when the fungus grows them.
fn hat_with_vines(view: &mut View, random: &mut Mt19937, [x, y, z]: [i32; 3], fungus: &Fungus, chance: i32) {
    view.set_block(x, y, z, fungus.hat);
    if fungus.vines && random.next_int(chance) == 0 && view.block(x, y - 1, z) == Block::Air {
        let length = vine_length(random, 5, 0);
        weeping_column(view, random, [x, y - 1, z], length, (23, 25));
    }
}
