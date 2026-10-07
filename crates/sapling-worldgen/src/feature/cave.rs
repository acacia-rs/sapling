//! The lush caves' wrapper features: `SnapToSurfaceFeature` (143bb7720), `VegetationPatchFeature`
//! (143bc8310) and `GrowingPlantFeature` (144c15200). Ported from stirante/featurelab
//! `features/{snap_to_surface,vegetation_patch,growing_plant}.go` (MIT, (c) 2026 stirante).

use super::cave_spec::{GrowingPlant, SnapToSurface, VegetationPatch};
use super::data::FEATURES;
use super::lush::{sturdy, wet};
use super::place::Ctx;
use super::view::View;
use crate::block::Block;
use crate::random::Mt19937;

/// `nextInt` of the summed weights (truncated after each float add), then the first entry the
/// remainder, also truncated per step, falls in; None when nothing is picked.
pub fn pick_weighted<T: Copy>(entries: &[(T, f32)], random: &mut Mt19937) -> Option<T> {
    let total = entries.iter().fold(0i32, |total, &(_, w)| (total as f32 + w) as i32);
    let mut left = if total == 0 { 0 } else { random.next_int(total) };
    for &(entry, weight) in entries {
        left = (left as f32 - weight) as i32;
        if left < 0 {
            return Some(entry);
        }
    }
    None
}

fn air(view: &View, [x, y, z]: [i32; 3]) -> bool {
    view.block(x, y, z) == Block::Air
}

impl SnapToSurface {
    /// Only from an air cell: walk while air (at most `range - 1` steps) to a sturdy block.
    pub fn place(&self, ctx: &mut Ctx, at: [i32; 3]) -> Option<[i32; 3]> {
        if !air(ctx.view, at) {
            return None;
        }
        let dy = if self.ceiling { 1 } else { -1 };
        let mut steps = 0;
        let mut cur = [at[0], at[1] + dy, at[2]];
        while steps < self.range - 1 && air(ctx.view, cur) {
            steps += 1;
            cur[1] += dy;
        }
        if !sturdy(ctx.view.block(cur[0], cur[1], cur[2])) {
            return None;
        }
        let snapped = [at[0], at[1] + dy * steps, at[2]];
        // Fitted to the oracle: BDS drops snaps that leave the origin's biome (lush caves ceilings).
        if (ctx.biome)(snapped) != (ctx.biome)(at) {
            return None;
        }
        FEATURES[self.feature].place(ctx, snapped)
    }
}

impl VegetationPatch {
    pub fn place(&self, ctx: &mut Ctx, at: [i32; 3]) -> Option<[i32; 3]> {
        let rx = self.radius.draw(ctx.random) + 1;
        let rz = self.radius.draw(ctx.random) + 1;
        let mut cells = self.ground(ctx.view, ctx.random, at, rx, rz);
        if self.waterlogged {
            cells.retain(|&c| !exposed(ctx.view, c));
            for &[x, y, z] in &cells {
                ctx.view.set_block(x, y, z, Block::Water);
            }
        }
        let &last = cells.last()?;
        let up = if self.ceiling { -1 } else { 1 };
        for &[x, y, z] in &cells {
            if self.vegetation_chance > 0.0 && ctx.random.next_float() < self.vegetation_chance {
                let y = if self.waterlogged { y - 1 } else { y };
                FEATURES[self.vegetation].place(ctx, [x, y + up, z]);
            }
        }
        Some(last)
    }

    /// The patch's ground cells: edges kept with `extra_edge_chance`, corners never; each column
    /// found within `vertical_range` and filled `depth` deep with the ground block.
    fn ground(&self, view: &mut View, random: &mut Mt19937, [ox, oy, oz]: [i32; 3], rx: i32, rz: i32) -> Vec<[i32; 3]> {
        let scan = if self.ceiling { 1 } else { -1 };
        let mut cells = Vec::new();
        for dx in -rx..=rx {
            let x_edge = dx.abs() == rx;
            for dz in -rz..=rz {
                let z_edge = dz.abs() == rz;
                if x_edge && z_edge {
                    continue;
                }
                if x_edge != z_edge && (self.extra_edge_chance == 0.0 || random.next_float() > self.extra_edge_chance) {
                    continue;
                }
                let mut p = [ox + dx, oy, oz + dz];
                if air(view, p) {
                    for n in 1.. {
                        p[1] += scan;
                        if !air(view, p) || n >= self.vertical_range {
                            break;
                        }
                    }
                }
                if !air(view, p) {
                    for n in 1.. {
                        p[1] -= scan;
                        if air(view, p) || n >= self.vertical_range {
                            break;
                        }
                    }
                }
                let ground = [p[0], p[1] + scan, p[2]];
                if !air(view, p) || !sturdy(view.block(ground[0], ground[1], ground[2])) {
                    continue;
                }
                let deep = i32::from(self.extra_deep_chance > 0.0 && random.next_float() < self.extra_deep_chance);
                let depth = self.depth.draw(random) + deep;
                let mut wrote = 0;
                let mut skip = false;
                for i in 0..depth {
                    let [x, y, z] = [ground[0], ground[1] + i * scan, ground[2]];
                    let block = view.block(x, y, z);
                    if block == self.ground {
                        wrote += 1;
                    } else if self.replaceable.contains(&block) {
                        view.set_block(x, y, z, self.ground);
                        wrote += 1;
                    } else {
                        skip = wrote == 0;
                        break;
                    }
                }
                if !skip {
                    cells.push(ground);
                }
            }
        }
        cells
    }
}

/// A pool cell any of whose four sides or bottom is not sturdy.
fn exposed(view: &View, [x, y, z]: [i32; 3]) -> bool {
    [(0, 0, -1), (1, 0, 0), (0, 0, 1), (-1, 0, 0), (0, -1, 0)].iter().any(|&(dx, dy, dz)| !sturdy(view.block(x + dx, y + dy, z + dz)))
}

impl GrowingPlant {
    /// Height and age drawn first; body blocks are picked per cell, the head (with the age) last.
    pub fn place(&self, ctx: &mut Ctx, [x, y, z]: [i32; 3]) -> Option<[i32; 3]> {
        let range = pick_weighted(self.heights, ctx.random).unwrap_or(super::tree::spec::Range(0, 0));
        let height = range.draw(ctx.random);
        let age = self.age.draw(ctx.random);
        if height < 1 {
            return None;
        }
        let step = if self.up { 1 } else { -1 };
        let mut head = None;
        for i in 0..height {
            let cy = y + i * step;
            let block = ctx.view.block(x, cy, z);
            if !(block == Block::Air || self.allow_water && block == Block::Water) {
                continue;
            }
            if i == height - 1 || ctx.view.block(x, cy + step, z) != Block::Air {
                head = Some([x, cy, z]);
                break;
            }
            if let Some(body) = pick_weighted(self.body, ctx.random) {
                put(ctx.view, [x, cy, z], body);
            }
        }
        let head_at = head?;
        if let Some(block) = pick_weighted(self.head, ctx.random) {
            let block = if self.age.1 != 0 { aged(block, age) } else { block };
            put(ctx.view, head_at, block);
        }
        Some(head_at)
    }
}

/// Waterloggable plants written into water keep it.
fn put(view: &mut View, [x, y, z]: [i32; 3], block: Block) {
    let block = if view.block(x, y, z) == Block::Water { wet(block) } else { block };
    view.set_block(x, y, z, block);
}

/// The cave vine head of `growing_plant_age` `age` (17..=25 exist).
fn aged(block: Block, age: i32) -> Block {
    let base = match block {
        Block::CaveVines0 => Block::CaveVines17,
        Block::CaveVinesHeadBerries17 => block,
        _ => return block,
    };
    Block::ALL[base as usize + (age.clamp(17, 25) - 17) as usize]
}
