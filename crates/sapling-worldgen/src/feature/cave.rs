//! The cave wrapper features: `SnapToSurfaceFeature` (143bb7720, from its decompile),
//! `VegetationPatchFeature` (143bc8310) and `GrowingPlantFeature` (144c15200), the last two ported
//! from stirante/featurelab `features/{vegetation_patch,growing_plant}.go` (MIT, (c) 2026 stirante).

use super::cave_spec::{GrowingPlant, SnapToSurface, Surface, VegetationPatch};
use super::data::FEATURES;
use super::lush::wet;
use super::place::Ctx;
use super::view::View;
use crate::block::{Block, Support};
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

/// FacingID offsets: down, up, north, south, west, east.
const FACING: [[i32; 3]; 6] = [[0, -1, 0], [0, 1, 0], [0, 0, -1], [0, 0, 1], [-1, 0, 0], [1, 0, 0]];

fn water(block: Block) -> bool {
    block.is_fluid() && !block.is_lava()
}

impl SnapToSurface {
    /// Ceilings and floors (`random_horizontal`: one of them by a boolean) search away from the origin's
    /// side of the surface (into the open from an open cell); walls (`random_vertical`) try the four
    /// sides in an order shuffled with raw draws.
    pub fn place(&self, ctx: &mut Ctx, at: [i32; 3]) -> Option<[i32; 3]> {
        super::trace::line(|| format!("  snap {} {} {} draw {}", at[0], at[1], at[2], ctx.random.draws()));
        let snapped = match self.surface {
            Surface::RandomVertical => {
                let mut order = [2, 5, 3, 4];
                for i in 1..4 {
                    let j = (ctx.random.next_u32() % (i as u32 + 1)) as usize;
                    order.swap(i, j);
                }
                order.iter().find_map(|&facing| self.find(ctx, at, facing))
            }
            surface => {
                let ceiling = match surface {
                    Surface::Ceiling => true,
                    Surface::Floor => false,
                    _ => ctx.random.next_boolean(),
                };
                let block = ctx.view.block(at[0], at[1], at[2]);
                let open = block == Block::Air || water(block);
                self.find(ctx, at, usize::from(ceiling == open))
            }
        };
        FEATURES[self.feature].place(ctx, snapped?)
    }

    /// `_findSnapPosInDirection` (143bb7ac0): `scanColumnWithinPosBiome` through cells of the
    /// origin's biome that are open (from an open origin) or solid (from a solid one) to a surface.
    fn find(&self, ctx: &Ctx, at: [i32; 3], facing: usize) -> Option<[i32; 3]> {
        let origin = ctx.view.block(at[0], at[1], at[2]);
        let solid = origin != Block::Air && !water(origin);
        let open = |b: Block| (self.allow_air && b == Block::Air) || (self.allow_water && water(b));
        let filled = |b: Block| self.allow_solid && b != Block::Air && !water(b);
        let inside = |b: Block| if solid { filled(b) } else { open(b) };
        let edge = |b: Block| {
            if !self.allowed.is_empty() {
                self.allowed.iter().any(|a| a.name() == b.name())
            } else if solid {
                open(b)
            } else {
                b.supports(facing ^ 1, Support::Any)
            }
        };
        let biome = (ctx.biome)(at);
        let [dx, dy, dz] = FACING[facing];
        let cell = |n: i32| [at[0] + dx * n, at[1] + dy * n, at[2] + dz * n];
        let block = |p: [i32; 3]| ctx.view.block(p[0], p[1], p[2]);
        if !inside(origin) {
            return None;
        }
        let mut steps = 0;
        while steps < self.range - 1 && (ctx.biome)(cell(steps + 1)) == biome && inside(block(cell(steps + 1))) {
            steps += 1;
        }
        edge(block(cell(steps + 1))).then(|| cell(steps + i32::from(solid != self.embed)))
    }
}

impl VegetationPatch {
    pub fn place(&self, ctx: &mut Ctx, at: [i32; 3]) -> Option<[i32; 3]> {
        super::trace::line(|| format!("  patch {} {} {} draw {}", at[0], at[1], at[2], ctx.random.draws()));
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
                let toward = usize::from(scan < 0);
                if !air(view, p) || !view.block(ground[0], ground[1], ground[2]).supports(toward, Support::Any) {
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
                    super::trace::line(|| format!("  cell {} {} {} depth {depth}", ground[0], ground[1], ground[2]));
                    cells.push(ground);
                }
            }
        }
        cells
    }
}

/// A pool cell any of whose four sides or bottom does not hold its face toward the cell.
fn exposed(view: &View, [x, y, z]: [i32; 3]) -> bool {
    [(0, 0, -1, 3), (1, 0, 0, 4), (0, 0, 1, 2), (-1, 0, 0, 5), (0, -1, 0, 1)]
        .iter()
        .any(|&(dx, dy, dz, face)| !view.block(x + dx, y + dy, z + dz).supports(face, Support::Any))
}

impl GrowingPlant {
    /// Height and age drawn first; body blocks are picked per cell, the head (with the age) last.
    pub fn place(&self, ctx: &mut Ctx, [x, y, z]: [i32; 3]) -> Option<[i32; 3]> {
        super::trace::line(|| format!("  plant {x} {y} {z} draw {}", ctx.random.draws()));
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
            if !(block == Block::Air || self.allow_water && water(block)) {
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
    Block::ALL[base.index() + (age.clamp(17, 25) - 17) as usize]
}
