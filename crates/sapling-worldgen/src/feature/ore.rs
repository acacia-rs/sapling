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
        self.fill(view, &kept, random)
    }

    fn fill(&self, view: &mut View, kept: &[Sphere], random: &mut Mt19937) -> bool {
        let floor = |v: f32| v.floor() as i32;
        let (mut lo, mut hi) = ([i32::MAX; 3], [i32::MIN; 3]);
        for sphere in kept {
            for axis in 0..3 {
                lo[axis] = lo[axis].min(floor(sphere.centre[axis] - sphere.radius));
                hi[axis] = hi[axis].max(floor(sphere.centre[axis] + sphere.radius));
            }
        }
        let mut placed = false;
        for x in lo[0]..=hi[0] {
            for z in lo[2]..=hi[2] {
                for y in lo[1]..=hi[1] {
                    let cell = [x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5];
                    if !kept.iter().any(|s| distance_sq(cell, s.centre) < s.radius_sq) {
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

fn exposed(view: &View, [x, y, z]: [i32; 3]) -> bool {
    FACES.iter().any(|[fx, fy, fz]| view.block(x + fx, y + fy, z + fz) == Block::Air)
}
