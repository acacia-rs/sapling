//! A map's drawing of the world around its holder: BDS's port of Java's `MapItem.update`
//! (decompiled, docs/research/maps.md "Drawing"). Each pixel takes the most common colour of the
//! highest coloured blocks of its scale's square, shaded by how its height compares with the pixel
//! north of it; water is always at full brightness.

use sapling_data::{MapColour, Tint, map_colour, tint_colour};
use sapling_world::{BlockRegistry, ChunkPos};

use super::MapData;

pub(super) const SIZE: i32 = 128;
const HALF: i32 = SIZE / 2;
const LOW: f32 = 0.705_882_4;
const NORMAL: f32 = 0.862_745_1;
const HIGH: f32 = 1.0;
/// What a pixel with no coloured block under it shows.
const NOTHING: u32 = 0xFF00_0000;
/// The Nether's ceiling hides its ground: maps there draw Java's noise of dirt and stone.
const NETHER: i32 = 1;
const NETHER_HEIGHT: i32 = 100;
const DIRT: [f32; 3] = [151.0, 109.0, 77.0];
const STONE: [f32; 3] = [112.0; 3];

/// Each runtime id's map colour, built for a registry.
#[derive(Default)]
pub(crate) struct Colours(Vec<Option<MapColour>>);

impl Colours {
    fn of(&mut self, registry: &BlockRegistry) -> &[Option<MapColour>] {
        if self.0.len() != registry.len() {
            self.0 = (0..registry.len() as u32).map(|id| registry.get(id).and_then(|state| map_colour(state.name))).collect();
        }
        &self.0
    }
}

/// A pixel as the wire carries it: alpha, blue, green, red from the top byte down.
pub(super) fn shaded(rgb: [f32; 3], by: f32) -> u32 {
    let [r, g, b] = rgb.map(|channel| ((channel / 255.0 * by) * 255.0) as u32);
    0xFF00_0000 | b << 16 | g << 8 | r
}

/// A block's colour where it stands, each channel 0 to 255, and whether it is water's.
fn tinted(colour: MapColour, biome: u32) -> ([f32; 3], bool) {
    let Some(tint) = colour.tint else { return (colour.rgb, false) };
    let by = tint_colour(biome, tint);
    (std::array::from_fn(|i| colour.rgb[i] / 255.0 * (by[i] / 255.0) * 255.0), tint == Tint::Water)
}

/// The most common colour of one pixel's square and its height, or `None` where nothing is loaded.
fn sample(world: &sapling_world::World, colours: &[Option<MapColour>], [wx, wz]: [i32; 2], scale: i32) -> Option<(Option<([f32; 3], bool)>, i32)> {
    if !world.is_loaded(ChunkPos::of_block(wx, wz)) {
        return None;
    }
    if world.dimension() == NETHER {
        let mut noise = wx.wrapping_add(wz.wrapping_mul(231_871));
        noise = noise.wrapping_mul(noise).wrapping_mul(31_287_121).wrapping_add(noise.wrapping_mul(11));
        return Some((Some((if noise >> 20 & 1 == 0 { DIRT } else { STONE }, false)), NETHER_HEIGHT));
    }
    let coloured = |id: u32| colours.get(id as usize).is_some_and(Option::is_some);
    let (mut counted, mut height): (Vec<(([f32; 3], bool), u32)>, i32) = (Vec::new(), 0);
    for dx in 0..scale {
        for dz in 0..scale {
            let (x, z) = (wx + dx, wz + dz);
            let Some(y) = world.top_where(x, z, coloured) else { continue };
            let Some(colour) = colours[world.block(x, y, z) as usize] else { continue };
            height += y / (scale * scale);
            let found = tinted(colour, world.biome(x, y, z).unwrap_or_default());
            match counted.iter_mut().find(|(seen, _)| seen.0.map(f32::to_bits) == found.0.map(f32::to_bits)) {
                Some((_, count)) => *count += 1,
                None => counted.push((found, 1)),
            }
        }
    }
    // The first of the most common, as Java's `copyHighestCountFirst`.
    let most = counted.iter().map(|(_, count)| *count).max();
    Some((counted.into_iter().find(|(_, count)| Some(*count) == most).map(|(colour, _)| colour), height))
}

/// Redraws the circle of `map` around a holder standing at `feet`.
pub(super) fn render(world: &sapling_world::World, colours: &mut Colours, map: &mut MapData, feet: [f32; 3]) {
    let colours = colours.of(world.registry());
    let scale = 1 << map.scale;
    let radius = SIZE / scale / if world.dimension() == NETHER { 2 } else { 1 };
    let [ox, oz] = map.centre;
    let [px, pz] = [(feet[0].floor() as i32 - ox) / scale + HALF, (feet[2].floor() as i32 - oz) / scale + HALF];
    for x in (px - radius + 1).max(0)..(px + radius).min(SIZE) {
        let mut north: Option<i32> = None;
        for z in (pz - radius - 1).max(-1)..(pz + radius).min(SIZE) {
            let (dx, dz) = (x - px, z - pz);
            let distance = dx * dx + dz * dz;
            let edge = distance > (radius - 2) * (radius - 2);
            let corner = [(ox / scale + x - HALF) * scale, (oz / scale + z - HALF) * scale];
            let Some((colour, height)) = sample(world, colours, corner, scale) else { continue };
            let step = (height - north.unwrap_or(height)) as f32 * 4.0 / (scale + 4) as f32 + (((x + z) & 1) as f32 - 0.5) * 0.4;
            north = Some(height);
            if z < 0 || distance >= radius * radius || (edge && (x + z) & 1 == 0) {
                continue;
            }
            let pixel = match colour {
                None => NOTHING,
                Some((rgb, true)) => shaded(rgb, HIGH),
                Some((rgb, false)) => shaded(rgb, if step > 0.6 { HIGH } else if step < -0.6 { LOW } else { NORMAL }),
            };
            map.paint(x as usize, z as usize, pixel);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixels_shade_as_bds_drew_them() {
        let grass = map_colour("minecraft:grass_block").unwrap();
        let (rgb, _) = tinted(grass, 1);
        assert_eq!(shaded(rgb, NORMAL), 0xFF4B_A27D);
        assert_eq!(shaded(rgb, LOW), 0xFF3E_8467);
        let water = tinted(map_colour("minecraft:water").unwrap(), 1);
        assert_eq!(shaded(water.0, HIGH), 0xFFF5_5A1E);
        assert!(water.1);
    }
}
