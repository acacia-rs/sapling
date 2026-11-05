//! The overworld generator's structures asked for by name, as `/locate structure` and
//! `/place structure` name them (`village`, `minecraft:trail_ruins`).

use sapling_worldgen::structure::StructureKind;
use sapling_worldgen::{OverworldGenerator, PlaceFailure, StructureWrites};

use crate::ChunkPos;
use crate::storage::ShapeLoader;

/// The kind `/locate structure` takes for `name`. BDS found no abandoned camp, so it is left out
/// (research/captures/bds-locate.txt).
fn kind(name: &str) -> Option<StructureKind> {
    let name = name.strip_prefix("minecraft:").unwrap_or(name);
    StructureKind::ALL.into_iter().find(|kind| kind.name() == name && *kind != StructureKind::AbandonedCamp)
}

/// The block BDS names for the nearest start: a stronghold's is where an eye of ender flies, the
/// others' as `OverworldGenerator::located_column` (research/captures/bds-locate.txt).
pub(super) fn locate(generator: &OverworldGenerator, name: &str, x: i32, z: i32) -> Option<[i32; 2]> {
    match kind(name)? {
        StructureKind::Stronghold => generator.locate_stronghold(x, z).map(|[x, _, z]| [x, z]),
        kind => generator.locate_structure(kind, x, z).map(|(cx, cz)| generator.located_column(kind, cx, cz)),
    }
}

pub(super) fn structure(generator: &OverworldGenerator, name: &str, start: ChunkPos, start_y: Option<i32>, load: &ShapeLoader<'_>) -> Result<StructureWrites, PlaceFailure> {
    let kind = kind(name).ok_or(PlaceFailure::Unknown)?;
    generator.place_structure_at(kind, (start.x, start.z), start_y, load)
}
