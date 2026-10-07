//! The hardcoded disks `minecraft:sand_feature`, `gravel_feature` (`SandFeature::place` 149d60440) and
//! `clay_feature` (`ClayFeature::place` 148287a70): one draw for the radius, then a flat cylinder of
//! replacements, x then z then y.

use super::spec::Disk;
use super::view::View;
use crate::random::Mt19937;

impl Disk {
    pub fn place(&self, view: &mut View, [px, py, pz]: [i32; 3], random: &mut Mt19937) -> bool {
        if !self.needs.contains(&view.block(px, py, pz)) {
            return false;
        }
        let radius = random.next_int(self.radius - 2) + 2;
        let mut placed = false;
        for x in px - radius..=px + radius {
            for z in pz - radius..=pz + radius {
                let (dx, dz) = (x - px, z - pz);
                if dx * dx + dz * dz > radius * radius {
                    continue;
                }
                for y in py - self.half_height..=py + self.half_height {
                    if self.replaces.contains(&view.block(x, y, z)) {
                        placed |= view.set_block(x, y, z, self.block);
                    }
                }
            }
        }
        placed
    }
}
