//! Ruined portals (`RuinedPortalFeature::createStructureStart` 14835c7d0, piece postProcess
//! 14835d070): BDS's take on Java 1.16's portal, all drawn in the start chunk's post-process and
//! placed through the 3x3 block source. Biome tags as BDS tests them (VanillaBiomeTags table):
//! ocean and swamp sink to the sea floor, desert half-buries, jungle and swamp set mossiness,
//! ice keeps lava from turning to magma, extreme hills and mountain raise underground ones.

use super::BoundingBox;
use super::canvas::Canvas;
use super::template::{Placement, Rotation, Template, named};
use crate::block::Block;
use crate::random::Mt19937;

const OCEAN: &[u32] = &[0, 24, 40, 41, 42, 43, 44, 45, 46, 47];
const DESERT: &[u32] = &[2];
const JUNGLE: &[u32] = &[21, 23, 48];
const SWAMP: &[u32] = &[6];
const ICE: &[u32] = &[12];
const HIGH: &[u32] = &[3, 34, 131];
const MIN_Y: i32 = -64;
const SEA_LEVEL: i32 = 63;
/// Netherrack chance by distance from the portal (14a927a68).
const SPREAD: [f32; 14] = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.9, 0.9, 0.8, 0.7, 0.6, 0.4, 0.2];

/// The portal of start chunk `start` where it reaches `canvas`'s chunk; `random` is the start
/// chunk's structure generator, freshly seeded for the feature; `biome` the start's (at the
/// chunk centre) and `biome_at` any block's.
pub fn place(canvas: &mut Canvas, random: &mut Mt19937, start: (i32, i32), biome: u32, biome_at: &dyn Fn(i32, i32, i32) -> u32) {
    let cold = ICE.contains(&biome);
    let surface = random.next_float() >= 0.5;
    let desert = DESERT.contains(&biome);
    let sea_floor = OCEAN.contains(&biome) || SWAMP.contains(&biome);
    let rotation = Rotation::from_index(random.next_int(4));
    let mirror = random.next_float() >= 0.5;
    let (x0, z0) = (start.0 * 16, start.1 * 16);
    // BDS's post-process BlockSource holds the start's 3x3 chunks; heights beyond are its min height.
    let top = |canvas: &Canvas, x: i32, z: i32| {
        if ((x >> 4) - start.0).abs() > 1 || ((z >> 4) - start.1).abs() > 1 { MIN_Y } else { surface_height(canvas, x, z, !sea_floor) }
    };
    let h0 = top(canvas, x0, z0);
    let name = if random.next_float() >= 0.05 { format!("portal_{}", random.next_int(10) + 1) } else { format!("giant_portal_{}", random.next_int(3) + 1) };
    let template = Template::get(&format!("ruined_portal/{name}"));
    let (sx, sy, sz) = template.size;
    let (dx, _, dz) = rotation.apply((sx, 0, if mirror { -sz } else { sz }));
    let mut b = BoundingBox::new(x0.min(x0 + dx), h0 - 1, z0.min(z0 + dz), x0.max(x0 + dx), h0 - 1 + sy, z0.max(z0 + dz));
    let middle = |a: i32, b: i32| a + (b - a + 1) / 2;
    let h = top(canvas, middle(b.x0, b.x1), middle(b.z0, b.z1));
    let mut y = h - 1;
    if surface {
        if desert {
            y = b.y0 + random.next_int(6) + (h - b.y1);
        }
    } else {
        let high = HIGH.contains(&biome_at(middle(b.x0, b.x1), middle(b.y0, b.y1), middle(b.z0, b.z1)));
        let base = if high { SEA_LEVEL + 7 } else { MIN_Y + 15 };
        y = h - 2 - (b.y1 - b.y0);
        if y > base {
            y = base + random.next_int(y - base + 1);
        }
    }
    b.move_y(y - b.y0);
    while MIN_Y + 15 < b.y0 {
        let corners = [(b.x0, b.z0), (b.x0, b.z1), (b.x1, b.z0), (b.x1, b.z1)];
        let held = corners.iter().filter(|&&(x, z)| supports(canvas.get(x, b.y0 - 1, z), sea_floor)).count();
        if held >= 3 {
            break;
        }
        b.move_y(-1);
    }
    spread_netherrack(canvas, random, &b, !surface || desert, &|canvas, x, z| top(canvas, x, z));
    let ignore_air = surface && (desert || random.next_float() >= 0.5);
    let mut placement = Placement::new(rotation, BoundingBox::new(i32::MIN, MIN_Y, i32::MIN, i32::MAX, 319, i32::MAX));
    placement.mirror = mirror;
    placement.seed = random.seed();
    placement.ignore = ignore_air.then_some("air");
    placement.flood = underwater(canvas, &b);
    let chests = placement.place(canvas, template, (x0, b.y0, z0));
    random.consume_count(chests as u32);
    for x in b.x0 + 1..b.x1 {
        for z in b.z0 + 1..b.z1 {
            if canvas.get(x, b.y0, z).name() == "netherrack" {
                nether_column(canvas, random, (x, b.y0 - 1, z));
            }
        }
    }
    let mossiness = match () {
        _ if placement.flood => 0.8,
        _ if desert => 0.0,
        _ if JUNGLE.contains(&biome) => 0.8,
        _ if SWAMP.contains(&biome) => 0.5,
        _ => 0.2,
    };
    super::ruined_portal_age::age(canvas, random, &b, mossiness, cold, sea_floor);
}

/// `BlockSource::getAboveTopSolidBlock` as vt+0x178 asks it: one above the highest solid block,
/// water counting as solid when `water`.
fn surface_height(canvas: &Canvas, x: i32, z: i32, water: bool) -> i32 {
    (MIN_Y..=319).rev().find(|&y| canvas.get(x, y, z).is_solid() || water && canvas.get(x, y, z) == Block::Water).map_or(MIN_Y, |y| y + 1)
}

/// A corner the portal may stand on: not leaves, and solid (water too off the sea floor).
fn supports(block: Block, sea_floor: bool) -> bool {
    !block.name().ends_with("leaves") && (block == Block::Water && !sea_floor || block.is_solid())
}

/// Any corner of the box in water floods the portal (air becomes water, blocks hold it).
fn underwater(canvas: &Canvas, b: &BoundingBox) -> bool {
    [b.x0, b.x1].iter().any(|&x| [b.y0, b.y1].iter().any(|&y| [b.z0, b.z1].iter().any(|&z| canvas.get(x, y, z) == Block::Water)))
}

/// Netherrack and magma strewn around the portal, falling off with distance.
fn spread_netherrack(canvas: &mut Canvas, random: &mut Mt19937, b: &BoundingBox, buried: bool, top: &dyn Fn(&Canvas, i32, i32) -> i32) {
    let n = (b.x1 - b.x0 + b.z1 - b.z0 + 2) / 4;
    let extra = random.next_int((-7).max(-n) + 8);
    let (ux, uz) = (b.x0 + (b.x1 - b.x0 + 1) / 2, b.z0 + (b.z1 - b.z0 + 1) / 2);
    for x in ux - 14..=ux + 14 {
        for z in uz - 14..=uz + 14 {
            let d = (ux - x).abs() + (uz - z).abs() + extra;
            if d >= 14 || random.next_float() >= SPREAD[d.max(0) as usize] {
                continue;
            }
            let hh = top(canvas, x, z) - 1;
            let y = if buried { hh.min(b.y0) } else { hh };
            if (b.y0 - y).abs() < 4 {
                canvas.set(x, y, z, nether_block(random));
                nether_column(canvas, random, (x, y - 1, z));
            }
        }
    }
}

fn nether_block(random: &mut Mt19937) -> Block {
    named(if random.next_float() < 0.05 { "magma" } else { "netherrack" }, "", false)
}

/// Netherrack from `pos` down, each further block on a coin flip, up to nine (148360050).
fn nether_column(canvas: &mut Canvas, random: &mut Mt19937, (x, y, z): (i32, i32, i32)) {
    canvas.set(x, y, z, nether_block(random));
    for k in 1..=8 {
        if random.next_float() >= 0.5 {
            break;
        }
        canvas.set(x, y - k, z, nether_block(random));
    }
}
