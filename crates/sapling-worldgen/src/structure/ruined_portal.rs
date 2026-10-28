//! Ruined portals (`RuinedPortalFeature::createStructureStart` 14835c7d0, piece postProcess
//! 14835d070): BDS's take on Java 1.16's portal, all drawn in the start chunk's post-process and
//! placed through the 3x3 block source. Biome tags as BDS tests them (VanillaBiomeTags table):
//! ocean and swamp sink to the sea floor, desert half-buries, jungle and swamp set mossiness,
//! ice keeps lava from turning to magma, extreme hills and mountain raise underground ones. In the
//! Nether (dimension id 1, [rbp+0x434]) the portal sits at a random height, keeps its air below the
//! sea, never mosses and ages to blackstone.

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
const NETHER_TOP: i32 = crate::nether::HEIGHT - 1;
/// Netherrack chance by distance from the portal (14a927a68).
const SPREAD: [f32; 14] = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.9, 0.9, 0.8, 0.7, 0.6, 0.4, 0.2];

/// What the dimension and the start's biome decide.
struct Site<'a> {
    nether: bool,
    min_y: i32,
    cold: bool,
    desert: bool,
    sea_floor: bool,
    /// The overworld start's biome (at the chunk centre) and any block's.
    biome: u32,
    biome_at: &'a dyn Fn(i32, i32, i32) -> u32,
}

/// The portal of start chunk `start` where it reaches `canvas`'s chunk; `random` is the start
/// chunk's structure generator, freshly seeded for the feature; `biome` the start's (at the chunk
/// centre) and `biome_at` any block's.
pub fn place(canvas: &mut Canvas, random: &mut Mt19937, start: (i32, i32), biome: u32, biome_at: &dyn Fn(i32, i32, i32) -> u32) {
    let (desert, sea_floor) = (DESERT.contains(&biome), OCEAN.contains(&biome) || SWAMP.contains(&biome));
    let site = Site { nether: false, min_y: MIN_Y, cold: ICE.contains(&biome), desert, sea_floor, biome, biome_at };
    place_at(canvas, random, start, &site);
}

/// `place` in the Nether, whose biomes carry none of the portal's tags.
pub fn place_nether(canvas: &mut Canvas, random: &mut Mt19937, start: (i32, i32)) {
    let hell = crate::nether::HELL as u32;
    let site = Site { nether: true, min_y: 0, cold: false, desert: false, sea_floor: false, biome: hell, biome_at: &|_, _, _| hell };
    place_at(canvas, random, start, &site);
}

fn place_at(canvas: &mut Canvas, random: &mut Mt19937, start: (i32, i32), site: &Site) {
    let surface = random.next_float() >= 0.5;
    let rotation = Rotation::from_index(random.next_int(4));
    let mirror = random.next_float() >= 0.5;
    let (x0, z0) = (start.0 * 16, start.1 * 16);
    // BDS's post-process BlockSource holds the start's 3x3 chunks; heights beyond are its min height.
    let top = |canvas: &Canvas, x: i32, z: i32| {
        if ((x >> 4) - start.0).abs() > 1 || ((z >> 4) - start.1).abs() > 1 {
            site.min_y
        } else if site.nether {
            under_roof_height(canvas, x, z, !site.sea_floor)
        } else {
            surface_height(canvas, x, z, !site.sea_floor)
        }
    };
    let h0 = top(canvas, x0, z0);
    let name = if random.next_float() >= 0.05 { format!("portal_{}", random.next_int(10) + 1) } else { format!("giant_portal_{}", random.next_int(3) + 1) };
    let template = Template::get(&format!("ruined_portal/{name}"));
    let (sx, sy, sz) = template.size;
    let (dx, _, dz) = rotation.apply((sx, 0, if mirror { -sz } else { sz }));
    let mut b = BoundingBox::new(x0.min(x0 + dx), h0 - 1, z0.min(z0 + dz), x0.max(x0 + dx), h0 - 1 + sy, z0.max(z0 + dz));
    let y = if site.nether { random.next_int(85) + site.min_y + 15 } else { overworld_y(canvas, random, &b, surface, site, &top) };
    b.move_y(y - b.y0);
    while site.min_y + 15 < b.y0 {
        let corners = [(b.x0, b.z0), (b.x0, b.z1), (b.x1, b.z0), (b.x1, b.z1)];
        let held = corners.iter().filter(|&&(x, z)| supports(canvas.get(x, b.y0 - 1, z), site.sea_floor)).count();
        if held >= 3 {
            break;
        }
        b.move_y(-1);
    }
    spread_netherrack(canvas, random, &b, !surface || site.desert, &|canvas, x, z| top(canvas, x, z));
    let ignore_air = site.nether && b.y0 < crate::nether::SEA_LEVEL || surface && (site.desert || random.next_float() >= 0.5);
    let mut placement = Placement::new(rotation, BoundingBox::new(i32::MIN, MIN_Y, i32::MIN, i32::MAX, 319, i32::MAX));
    placement.mirror = mirror;
    placement.seed = random.seed();
    placement.ignore = ignore_air.then_some("air");
    placement.flood = underwater(canvas, &b);
    let chests = placement.place(canvas, template, (x0, b.y0, z0));
    chests.draw_seeds(canvas, random);
    for x in b.x0 + 1..b.x1 {
        for z in b.z0 + 1..b.z1 {
            if canvas.get(x, b.y0, z).name() == "netherrack" {
                nether_column(canvas, random, (x, b.y0 - 1, z));
            }
        }
    }
    let mossiness = match () {
        _ if site.nether => 0.0,
        _ if placement.flood => 0.8,
        _ if site.desert => 0.0,
        _ if JUNGLE.contains(&site.biome) => 0.8,
        _ if SWAMP.contains(&site.biome) => 0.5,
        _ => 0.2,
    };
    super::ruined_portal_age::age(canvas, random, &b, mossiness, site.cold, site.sea_floor, site.nether);
}

/// The overworld's portal bottom: on the surface (half buried in deserts), else sunk below it
/// to a random depth above the floor (or above sea level + 7 in high biomes).
fn overworld_y(canvas: &Canvas, random: &mut Mt19937, b: &BoundingBox, surface: bool, site: &Site, top: &dyn Fn(&Canvas, i32, i32) -> i32) -> i32 {
    let middle = |a: i32, b: i32| a + (b - a + 1) / 2;
    let h = top(canvas, middle(b.x0, b.x1), middle(b.z0, b.z1));
    if surface {
        return if site.desert { b.y0 + random.next_int(6) + (h - b.y1) } else { h - 1 };
    }
    let high = HIGH.contains(&(site.biome_at)(middle(b.x0, b.x1), middle(b.y0, b.y1), middle(b.z0, b.z1)));
    let base = if high { SEA_LEVEL + 7 } else { MIN_Y + 15 };
    let y = h - 2 - (b.y1 - b.y0);
    if y > base { base + random.next_int(y - base + 1) } else { y }
}

fn blocking(block: Block, water: bool) -> bool {
    block.is_solid() || water && block == Block::Water
}

/// `BlockSource::getAboveTopSolidBlock` as vt+0x178 asks it: one above the highest solid block,
/// water counting as solid when `water`.
fn surface_height(canvas: &Canvas, x: i32, z: i32, water: bool) -> i32 {
    (MIN_Y..=319).rev().find(|&y| blocking(canvas.get(x, y, z), water)).map_or(MIN_Y, |y| y + 1)
}

/// The same under a roof (`LevelChunk::getAboveTopSolidBlock` 140a08bb0 for a dimension without
/// sky): from the top down through the solid roof, then down to the first solid block.
fn under_roof_height(canvas: &Canvas, x: i32, z: i32, water: bool) -> i32 {
    let mut y = NETHER_TOP;
    while y > 0 && blocking(canvas.get(x, y, z), water) {
        y -= 1;
    }
    while y >= 0 && !blocking(canvas.get(x, y, z), water) {
        y -= 1;
    }
    (y + 1).max(0)
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
