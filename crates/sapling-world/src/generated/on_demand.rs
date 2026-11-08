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
/// others' as `OverworldGenerator::located_column` (research/captures/bds-locate.txt). An explorer
/// map's `village_<type>` takes the first village of that type, its `jungle_temple` or `swamp_hut` the
/// first temple of that kind; namespaced, as `/locate` asks, BDS found neither (bds-ax1maps.txt).
pub(super) fn locate(generator: &OverworldGenerator, name: &str, x: i32, z: i32) -> Option<[i32; 2]> {
    if let Some(village_type) = name.strip_prefix("village_") {
        return generator.locate_village(village_type, x, z).map(|(cx, cz)| generator.located_column(StructureKind::Village, cx, cz));
    }
    if let Some((cx, cz)) = generator.locate_temple(name, x, z) {
        return Some(generator.located_column(StructureKind::Temple, cx, cz));
    }
    match kind(name)? {
        StructureKind::Stronghold => generator.locate_stronghold(x, z).map(|[x, _, z]| [x, z]),
        kind => generator.locate_structure(kind, x, z).map(|(cx, cz)| generator.located_column(kind, cx, cz)),
    }
}

/// `/place structure` takes only the data-driven kinds: BDS refused `village`, `ancient_city` and
/// `mineshaft` (research/captures/bds-place.txt, bds-t1dplace.txt) and placed trail ruins and trial chambers.
// TODO: BDS also places an abandoned camp by its variant's name (`abandoned_camp_forest`).
pub(super) fn structure(generator: &OverworldGenerator, name: &str, start: ChunkPos, start_y: Option<i32>, load: &ShapeLoader<'_>) -> Result<StructureWrites, PlaceFailure> {
    let kind = kind(name).filter(|kind| matches!(kind, StructureKind::TrailRuins | StructureKind::TrialChambers)).ok_or(PlaceFailure::Unknown)?;
    generator.place_structure_at(kind, (start.x, start.z), start_y, load)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use sapling_worldgen::ChunkShape;

    use super::*;

    #[test]
    fn place_structure_takes_trail_ruins_and_trial_chambers_but_no_hardcoded_kind() {
        let generator = OverworldGenerator::new(20261003);
        let air = |_: i32, _: i32| Arc::new(ChunkShape::empty());
        let unknown = |name: &str| matches!(structure(&generator, name, ChunkPos::new(0, 0), Some(-20), &air), Err(PlaceFailure::Unknown));
        assert!(["minecraft:village", "minecraft:ancient_city", "minecraft:mineshaft", "minecraft:no_such_structure"].into_iter().all(unknown));
        assert!(!unknown("minecraft:trail_ruins") && !unknown("minecraft:trial_chambers"));
    }
}
