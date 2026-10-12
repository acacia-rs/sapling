//! The hardcoded disks `minecraft:sand_feature`, `gravel_feature` (`SandFeature::place` 149d60440),
//! `clay_feature` (`ClayFeature::place` 148287a70) and `grass_disc_feature` (148299570): one draw
//! for the radius, then a flat cylinder of replacements, x then z then y.

use super::spec::Disk;
use super::view::View;
use crate::block::Block;
use crate::random::Mt19937;

impl Disk {
    pub fn place(&self, view: &mut View, [px, py, pz]: [i32; 3], random: &mut Mt19937) -> bool {
        // `needs` water means its material (`ClayFeature::place` 148287a70, `SandFeature::place` 149d60440): any depth.
        let here = view.block(px, py, pz);
        if !self.needs.iter().any(|&need| need == here || need == Block::Water && here.is_water()) {
            return false;
        }
        let radius = random.next_int(self.radius - 2) + 2;
        let mut placed = false;
        for_disk_cell([px, py, pz], radius, self.half_height, |x, y, z| {
            if self.replaces.contains(&view.block(x, y, z)) {
                placed |= view.set_block(x, y, z, self.block);
            }
        });
        placed
    }
}

/// The mangrove swamp's grass disc (radius 5 in its ctor): dirt, coarse dirt, grass and mud become
/// grass under air, dirt elsewhere; no test at the centre, always succeeds.
pub fn grass_disc(view: &mut View, random: &mut Mt19937, at: [i32; 3]) -> bool {
    let radius = random.next_int(5 - 2) + 2;
    for_disk_cell(at, radius, 2, |x, y, z| {
        if matches!(view.block(x, y, z), Block::Dirt | Block::CoarseDirt | Block::GrassBlock | Block::Mud) {
            let block = if view.block(x, y + 1, z) == Block::Air { Block::GrassBlock } else { Block::Dirt };
            view.set_block(x, y, z, block);
        }
    });
    true
}

fn for_disk_cell([px, py, pz]: [i32; 3], radius: i32, half_height: i32, mut f: impl FnMut(i32, i32, i32)) {
    for x in px - radius..=px + radius {
        for z in pz - radius..=pz + radius {
            let (dx, dz) = (x - px, z - pz);
            if dx * dx + dz * dz > radius * radius {
                continue;
            }
            for y in py - half_height..=py + half_height {
                f(x, y, z);
            }
        }
    }
}
