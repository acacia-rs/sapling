//! `minecraft:ore_feature`: Java's legacy ore vein in f32 with Bedrock's sine table. Draws: the
//! angle, two y offsets, one float per sphere, then one float per replace-rule match while
//! `0 < discard_on_air < 1`. Adapted from featurelab's features/ore.go (MIT, stirante).

use super::spec::Ore;
use super::view::View;
use crate::block::Block;
use crate::mth;
use crate::random::Mt19937;

const PI: f32 = std::f32::consts::PI;
const FACES: [[i32; 3]; 6] = [[1, 0, 0], [-1, 0, 0], [0, 1, 0], [0, -1, 0], [0, 0, 1], [0, 0, -1]];

#[derive(Clone, Copy)]
struct Sphere {
    centre: [f32; 3],
    radius_sq: f32,
    radius: f32,
}

/// The vein's line from its first end. BDS keeps only cells within the widest sphere's radius of
/// it before testing the spheres (143b8dcb0); far from the origin the sphere centres round off the
/// line, so the gate drops cells the spheres hold.
struct Axis {
    start: [f32; 3],
    unit: [f32; 3],
    /// The unit vector's x and z over its horizontal length.
    flat: [f32; 2],
}

impl Axis {
    fn new(start: [f32; 3], [dx, dy, dz]: [f32; 3]) -> Self {
        let length = (dz * dz + dy * dy + dx * dx).sqrt();
        let unit = if length < 0.0001 { [0.0; 3] } else { [dx / length, dy / length, dz / length] };
        let flat_length = (unit[2] * unit[2] + unit[0] * unit[0]).sqrt();
        let flat = if flat_length < 0.0001 { [0.0; 2] } else { [unit[0] / flat_length, unit[2] / flat_length] };
        Self { start, unit, flat }
    }

    fn column_near(&self, cx: f32, cz: f32, widest: f32) -> bool {
        let [x1, _, z1] = self.start;
        let [hx, hz] = self.flat;
        let along = hz * (cz - z1) + hx * (cx - x1);
        let px = cx - (hx * along + x1);
        let pz = cz - (along * hz + z1);
        pz * pz + px * px <= widest
    }

    fn cell_near(&self, [cx, cy, cz]: [f32; 3], widest: f32) -> bool {
        let [x1, y1, z1] = self.start;
        let [ux, uy, uz] = self.unit;
        let along = (cy - y1) * uy + (cx - x1) * ux + (cz - z1) * uz;
        let qx = cx - (ux * along + x1);
        let qy = cy - (uy * along + y1);
        let qz = cz - (along * uz + z1);
        qz * qz + qy * qy + qx * qx <= widest
    }
}

impl Ore {
    /// Whether any block was placed.
    pub fn place(&self, view: &mut View, origin: [i32; 3], random: &mut Mt19937) -> bool {
        let angle = random.next_float() * PI;
        let (sin, cos) = (mth::sin(angle), mth::cos(angle));
        let count = self.count as f32;
        let inverse = 1.0 / count;
        let size = count / 8.0;
        let (axis_x, axis_z) = (sin * size, cos * size);
        let [ox, oy, oz] = origin.map(|v| v as f32);
        let x1 = ox + (8.0 + axis_x);
        let z1 = oz + (8.0 + axis_z);
        let y1 = oy + (random.next_int(3) as f32 - 2.0);
        let x2 = ox + (8.0 - axis_x);
        let z2 = oz + (8.0 - axis_z);
        let y2 = oy + (random.next_int(3) as f32 - 2.0);
        if self.rules.is_empty() {
            return false;
        }
        let (dx, dy, dz) = (x2 - x1, y2 - y1, z2 - z1);
        let axis = Axis::new([x1, y1, z1], [dx, dy, dz]);
        let step = PI * inverse;
        let spheres: Vec<Sphere> = (0..self.count)
            .map(|i| {
                let fi = i as f32;
                let centre = [x1 + dx * fi * inverse, y1 + dy * fi * inverse, z1 + dz * fi * inverse];
                let scale = random.next_float() * count / 16.0;
                let radius = ((mth::sin(fi * step) + 1.0) * scale + 1.0) / 2.0;
                let radius_sq = radius * radius;
                Sphere { centre, radius_sq, radius: radius_sq.sqrt() }
            })
            .collect();
        let kept = prune(spheres);
        self.fill(view, &kept, &axis, random)
    }

    fn fill(&self, view: &mut View, kept: &[Sphere], axis: &Axis, random: &mut Mt19937) -> bool {
        let floor = |v: f32| v.floor() as i32;
        let (mut lo, mut hi) = ([i32::MAX; 3], [i32::MIN; 3]);
        for sphere in kept {
            for axis in 0..3 {
                lo[axis] = lo[axis].min(floor(sphere.centre[axis] - sphere.radius));
                hi[axis] = hi[axis].max(floor(sphere.centre[axis] + sphere.radius));
            }
        }
        let widest = kept.iter().fold(0.0f32, |widest, s| if s.radius_sq <= widest { widest } else { s.radius_sq });
        let mut placed = false;
        let mut near: Vec<&Sphere> = Vec::with_capacity(kept.len());
        for x in lo[0]..=hi[0] {
            for z in lo[2]..=hi[2] {
                let (cx, cz) = (x as f32 + 0.5, z as f32 + 0.5);
                if !axis.column_near(cx, cz, widest) {
                    continue;
                }
                // A rounded sum of squares is no less than any of its terms, so a sphere whose x or z
                // square alone reaches its radius holds no cell of the column.
                let square = |a: f32, b: f32| (a - b) * (a - b);
                near.clear();
                near.extend(kept.iter().filter(|s| square(cx, s.centre[0]) < s.radius_sq && square(cz, s.centre[2]) < s.radius_sq));
                if near.is_empty() {
                    continue;
                }
                for y in lo[1]..=hi[1] {
                    let cell = [cx, y as f32 + 0.5, cz];
                    if !axis.cell_near(cell, widest) || !near.iter().any(|s| distance_sq(cell, s.centre) < s.radius_sq) {
                        continue;
                    }
                    placed |= self.replace(view, [x, y, z], random);
                }
            }
        }
        placed
    }

    /// The first replace rule whose `may_replace` holds the block and whose target it is not yet.
    fn replace(&self, view: &mut View, [x, y, z]: [i32; 3], random: &mut Mt19937) -> bool {
        let block = view.block(x, y, z);
        for rule in self.rules {
            if !rule.may_replace.contains(&block) || block == rule.places {
                continue;
            }
            let discard = self.discard_on_air;
            if discard > 0.0 && (discard >= 1.0 || random.next_float() < discard) && exposed(view, [x, y, z]) {
                continue;
            }
            if view.set_block(x, y, z, rule.places) {
                return true;
            }
        }
        false
    }
}

/// Drops each sphere wholly inside a later one; the last is always kept.
fn prune(spheres: Vec<Sphere>) -> Vec<Sphere> {
    let mut kept = Vec::with_capacity(spheres.len());
    for (j, sphere) in spheres.iter().enumerate() {
        let enclosed = spheres[j + 1..].iter().any(|later| sphere.radius + distance_sq(later.centre, sphere.centre).sqrt() < later.radius);
        if !enclosed {
            kept.push(*sphere);
        }
    }
    kept
}

/// `(dx*dx + dy*dy) + dz*dz`, `a - b` per axis.
fn distance_sq(a: [f32; 3], b: [f32; 3]) -> f32 {
    let (dx, dy, dz) = (a[0] - b[0], a[1] - b[1], a[2] - b[2]);
    dx * dx + dy * dy + dz * dz
}

pub(super) fn exposed(view: &View, [x, y, z]: [i32; 3]) -> bool {
    FACES.iter().any(|[fx, fy, fz]| view.block(x + fx, y + fy, z + fz) == Block::Air)
}
