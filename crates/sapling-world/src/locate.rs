//! Finding the generator's structures and biomes from a place, as `/locate` does.

use crate::World;

/// How far a biome search reaches, and its step, in blocks: Java's `/locate biome` values.
// TODO: BDS's own search is not known; its answers lay on a 32-block grid (bds-locate.txt).
const BIOME_REACH: i32 = 6400;
const BIOME_STEP: i32 = 32;
/// The heights a biome is looked for at, so that cave biomes are found too.
const BIOME_HEIGHTS: [i32; 4] = [80, 16, -48, 144];

impl World {
    /// The block column `/locate structure` answers for the structure `name` from block (x, z)
    /// ([`crate::WorldStorage::locate_structure`]); `None` without a generator or where none is found.
    pub fn locate_structure(&self, name: &str, x: i32, z: i32) -> Option<[i32; 2]> {
        self.loader.as_ref()?.storage().locate_structure(name, x, z)
    }

    /// The nearest cell of the biome `biome` to `from` on a 32-block grid, ring by ring outwards;
    /// `None` without a generator or where none is within reach.
    pub fn locate_biome(&self, biome: u32, from: [i32; 3]) -> Option<[i32; 3]> {
        let storage = self.loader.as_ref()?.storage();
        storage.generated_biome(from[0], from[1], from[2])?;
        let (cx, cz) = (from[0].div_euclid(BIOME_STEP), from[2].div_euclid(BIOME_STEP));
        for ring in 0..=BIOME_REACH / BIOME_STEP {
            let cells = (-ring..=ring).flat_map(|dx| (-ring..=ring).map(move |dz| (dx, dz))).filter(|(dx, dz)| dx.abs() == ring || dz.abs() == ring);
            let found = cells.filter_map(|(dx, dz)| {
                let (x, z) = ((cx + dx) * BIOME_STEP, (cz + dz) * BIOME_STEP);
                let y = BIOME_HEIGHTS.into_iter().find(|&y| storage.generated_biome(x, y, z) == Some(biome))?;
                Some([x, y, z])
            });
            if let Some(nearest) = found.min_by_key(|[x, _, z]| (i64::from(x - from[0])).pow(2) + (i64::from(z - from[2])).pow(2)) {
                return Some(nearest);
            }
        }
        None
    }
}
