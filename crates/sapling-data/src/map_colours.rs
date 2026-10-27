//! Each block's colour on a map, and the tints biomes lay on some, as BDS draws them
//! (`data/map-colours.tsv`, `data/map-tints.tsv`; README.md "Map colours").

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::spawn_rules::biome_id;

const COLOURS: &str = include_str!("../data/map-colours.tsv");
const TINTS: &str = include_str!("../data/map-tints.tsv");
/// BDS's birch and spruce leaves: one colour wherever they grow.
const BIRCH: [f32; 3] = [128.0, 167.0, 85.0];
const EVERGREEN: [f32; 3] = [97.0, 153.0, 97.0];

/// The biome colour a block's own is multiplied by (BDS's `minecraft:map_color` tint methods).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tint {
    Grass,
    Foliage,
    Birch,
    Evergreen,
    Water,
}

/// A block's colour on a map, each channel 0 to 255; not always a whole number.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapColour {
    pub rgb: [f32; 3],
    pub tint: Option<Tint>,
}

fn channels(text: &str) -> Option<[f32; 3]> {
    if let Some(parts) = text.split(',').map(|part| part.parse().ok()).collect::<Option<Vec<f32>>>().filter(|parts| parts.len() == 3) {
        return Some([parts[0], parts[1], parts[2]]);
    }
    let packed = u32::from_str_radix(text, 16).ok().filter(|_| text.len() == 6)?;
    Some([16, 8, 0].map(|shift| (packed >> shift & 0xff) as f32))
}

fn rows(text: &'static str) -> impl Iterator<Item = Vec<&'static str>> {
    text.lines().filter(|line| !line.starts_with('#') && !line.is_empty()).map(|line| line.split('\t').collect())
}

static TABLE: LazyLock<HashMap<&'static str, MapColour>> = LazyLock::new(|| {
    let tint = |name: &str| match name {
        "grass" => Some(Tint::Grass),
        "foliage" => Some(Tint::Foliage),
        "birch" => Some(Tint::Birch),
        "evergreen" => Some(Tint::Evergreen),
        "water" => Some(Tint::Water),
        _ => None,
    };
    rows(COLOURS).filter_map(|row| Some((row[0], MapColour { rgb: channels(row[1])?, tint: tint(row[2]) }))).collect()
});

/// Each biome's grass, foliage and water colours, by the id chunks store.
static BIOME_TINTS: LazyLock<HashMap<u32, [[f32; 3]; 3]>> =
    LazyLock::new(|| rows(TINTS).filter_map(|row| Some((biome_id(row[0])?, [channels(row[1])?, channels(row[2])?, channels(row[3])?]))).collect());

/// What the block `name` shows on a map; `None` for one maps look through (air, glass, torches).
pub fn map_colour(name: &str) -> Option<MapColour> {
    TABLE.get(name).copied()
}

/// The colour `tint` lays on a block in `biome` (an id chunks store), each channel 0 to 255:
/// plains' where the biome is not known.
pub fn tint_colour(biome: u32, tint: Tint) -> [f32; 3] {
    const PLAINS: u32 = 1;
    let [grass, foliage, water] = BIOME_TINTS.get(&biome).or_else(|| BIOME_TINTS.get(&PLAINS)).copied().unwrap_or([[255.0; 3]; 3]);
    match tint {
        Tint::Grass => grass,
        Tint::Foliage => foliage,
        Tint::Birch => BIRCH,
        Tint::Evergreen => EVERGREEN,
        Tint::Water => water,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_come_from_bds_maps() {
        assert_eq!(map_colour("minecraft:stone"), Some(MapColour { rgb: [112.0; 3], tint: None }));
        assert_eq!(map_colour("minecraft:grass_block").map(|colour| colour.tint), Some(Some(Tint::Grass)));
        assert_eq!(map_colour("minecraft:glass"), None);
        assert_eq!(map_colour("minecraft:element_1").map(|colour| colour.rgb), Some([122.43, 178.75, 68.69]));
        assert_eq!(tint_colour(1, Tint::Grass), [146.0, 188.0, 88.0]);
        assert_eq!(tint_colour(1, Tint::Water), [68.0, 175.0, 245.0]);
    }
}
