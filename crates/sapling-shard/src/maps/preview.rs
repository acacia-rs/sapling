//! An explorer map's preview of its square, drawn as it is made: BDS's `MapItem::renderBiomePreviewMap`
//! (tiles 141a68d40, each drawn by 141a756f0), Java's of the same name with the stripes along rows.
//! A pixel's biome is water when its depth is below 0; water by land is outlined orange, the land by
//! it brown, open water striped. Nothing is drawn in a flat world (14161b3e0).

use sapling_world::World;

use super::MapData;
use super::render::SIZE;

const TILE: usize = 16;
/// Brightness by index: low, normal, high, lowest (BDS 0x14a76dd60).
const BRIGHTNESS: [f32; 4] = [180.0, 220.0, 255.0, 135.0];
const LOW: usize = 0;
const NORMAL: usize = 1;
const HIGH: usize = 2;
const LOWEST: usize = 3;
const ORANGE: [f32; 3] = [0.847_058_83, 0.498_039_22, 0.2];
const BROWN: [f32; 3] = [0.4, 0.298_039_23, 0.2];

/// The height the biomes are read at: the map's own, which its packets give as 0.
const Y: i32 = 0;

/// Draws the preview into each tile of `map` still blank.
pub(super) fn preview(world: &World, map: &mut MapData) {
    if world.level().flat || world.generated_biome(map.centre[0], Y, map.centre[1]).is_none() {
        return;
    }
    let size = SIZE as usize;
    for tile_z in (0..size).step_by(TILE) {
        for tile_x in (0..size).step_by(TILE) {
            let blank = (tile_z..tile_z + TILE).all(|z| map.pixels[z * size + tile_x..z * size + tile_x + TILE].iter().all(|pixel| *pixel == 0));
            if blank {
                draw_tile(world, map, [tile_x, tile_z]);
            }
        }
    }
}

fn draw_tile(world: &World, map: &mut MapData, [x0, z0]: [usize; 2]) {
    let step = 1 << map.scale;
    let [left, top] = map.centre.map(|centre| (centre / step - SIZE / 2) * step);
    // Each pixel's biome is water or not, with a pixel's border round the tile.
    let side = TILE + 2;
    let water: Vec<bool> = (0..side * side)
        .map(|at| {
            let (col, row) = ((at % side) as i32 - 1 + x0 as i32, (at / side) as i32 - 1 + z0 as i32);
            world.generated_biome(left + col * step, Y, top + row * step).is_some_and(|biome| sapling_data::biome_depth(biome) < 0.0)
        })
        .collect();
    for row in 0..TILE {
        for col in 0..TILE {
            let wet = |dx: usize, dz: usize| water[(row + dz) * side + col + dx];
            let around = [(0, 0), (1, 0), (2, 0), (0, 1), (2, 1), (0, 2), (1, 2), (2, 2)].iter().filter(|(dx, dz)| wet(*dx, *dz)).count();
            if let Some((rgb, brightness)) = pixel(wet(1, 1), around, row, col) {
                map.paint(x0 + col, z0 + row, shade(rgb, brightness));
            }
        }
    }
}

/// A pixel's colour and brightness from whether it is water and how many of its eight neighbours
/// are; `None` leaves it blank.
fn pixel(water: bool, around: usize, row: usize, col: usize) -> Option<([f32; 3], usize)> {
    if !water {
        return match around {
            0 => None,
            1..=3 => Some((BROWN, LOWEST)),
            _ => Some((BROWN, NORMAL)),
        };
    }
    if around == 8 {
        return (row % 2 == 1).then(|| (ORANGE, stripe(row, col)));
    }
    Some((ORANGE, match around {
        6.. => NORMAL,
        2.. => LOW,
        _ => LOWEST,
    }))
}

/// Open water's stripe on an odd row: bands of 8 along it, shifted by the row's sine.
fn stripe(row: usize, col: usize) -> usize {
    let shift = (sapling_worldgen::mth::sin((row + 1) as f32) * 7.0) as i32;
    let band = (col as u32).wrapping_add(shift as u32).wrapping_add(1) >> 3;
    match band % 5 {
        3 => NORMAL,
        4 => LOW,
        band => band as usize,
    }
}

/// A colour at a brightness as the wire carries it, each channel cut as BDS cuts it.
fn shade(rgb: [f32; 3], brightness: usize) -> u32 {
    let by = BRIGHTNESS[brightness] / 255.0;
    let [r, g, b] = rgb.map(|channel| ((channel * by).clamp(0.0, 1.0) * 255.0) as u32);
    0xFF00_0000 | b << 16 | g << 8 | r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_match_bds_pixels() {
        // bds-at1maps.txt: the taiga map's first row.
        assert_eq!([shade(BROWN, LOWEST), shade(ORANGE, LOW), shade(ORANGE, NORMAL), shade(BROWN, NORMAL), shade(ORANGE, HIGH), shade(ORANGE, LOWEST)].map(|pixel| pixel as i32), [-14997450, -14394984, -13865542, -13876904, -13402152, -14990478]);
    }

    /// A row as the drill prints it: runs of `count*rrggbbaa`.
    fn runs(row: &[u32]) -> String {
        let mut out: Vec<(usize, u32)> = Vec::new();
        for &pixel in row {
            match out.last_mut() {
                Some((count, last)) if *last == pixel => *count += 1,
                _ => out.push((1, pixel)),
            }
        }
        out.iter().map(|(count, pixel)| format!("{count}*{:08x}", pixel.swap_bytes())).collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn the_taiga_village_map_previews_as_bds_drew_it() {
        // research/captures/bds-ax1maps.txt: the map of the taiga village (-984, 136), rows 0, 1, 13, 64
        // (all 128 rows matched once; read at y 64 rows 52 to 70 and 91 on differed).
        let world = World::with_storage(std::sync::Arc::new(sapling_world::Generated::with_areas(sapling_world::MemoryStorage::default(), 20261003, 1)));
        let mut map = MapData::kept(0, 2, [-832, 192], 0, bevy_ecs::entity::Entity::PLACEHOLDER, false, vec![0; SIZE as usize * SIZE as usize]);
        preview(&world, &mut map);
        let row = |z: usize| runs(&map.pixels[z * SIZE as usize..][..SIZE as usize]);
        assert_eq!(row(0), "23*00000000 1*36281bff 1*985924ff 5*00000000 1*ba6d2cff 1*58412cff 3*36281bff 1*985924ff 1*ba6d2cff 70*00000000 1*985924ff 1*36281bff 19*00000000");
        assert_eq!(
            row(1),
            "23*00000000 1*36281bff 1*985924ff 1*ba6d2cff 4*d87f33ff 1*ba6d2cff 1*985924ff 1*36281bff 1*00000000 1*36281bff 1*58412cff 5*ba6d2cff 7*d87f33ff 1*985924ff 8*ba6d2cff 7*d87f33ff 1*985924ff 8*ba6d2cff 7*d87f33ff 1*985924ff 8*ba6d2cff 7*d87f33ff 1*985924ff 8*ba6d2cff 2*d87f33ff 1*985924ff 1*36281bff 19*00000000"
        );
        assert_eq!(
            row(13),
            "29*00000000 2*36281bff 2*985924ff 8*ba6d2cff 7*d87f33ff 1*985924ff 8*ba6d2cff 7*d87f33ff 1*985924ff 8*ba6d2cff 7*d87f33ff 1*985924ff 8*ba6d2cff 7*d87f33ff 1*985924ff 6*ba6d2cff 1*985924ff 2*36281bff 22*00000000"
        );
        assert_eq!(
            row(64),
            "2*00000000 1*985924ff 1*36281bff 96*00000000 1*36281bff 1*985924ff 2*58412cff 2*985924ff 2*ba6d2cff 4*00000000 2*ba6d2cff 2*985924ff 2*ba6d2cff 1*00000000 2*ba6d2cff 2*985924ff 2*36281bff 3*00000000"
        );
    }
}
