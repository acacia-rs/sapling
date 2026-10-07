//! Starts BDS 1.26.52 gave for seed 20261003: `/locate structure <kind>` from 0 64 0.

use super::StructureKind::{self, *};
use crate::OverworldGenerator;

/// (kind, block x, block z) of each answer.
const ANSWERS: [(StructureKind, i32, i32); 12] = [
    (Village, -984, 136),
    (Mineshaft, -272, -16),
    (Stronghold, -1422, -1950),
    (Monument, -360, -344),
    (Mansion, -8184, 15944),
    (OceanRuins, 56, 104),
    (Shipwreck, -104, -376),
    (BuriedTreasure, 136, 200),
    (Outpost, 1896, -1112),
    (RuinedPortal, 184, 296),
    (Temple, 680, -424),
    (AncientCity, -1112, 1272),
];

#[test]
fn bds_locate_answers_are_starts() {
    let generator = OverworldGenerator::new(20261003);
    for (kind, x, z) in ANSWERS {
        let chunk = (x.div_euclid(16), z.div_euclid(16));
        assert!(generator.is_structure_start(kind, chunk.0, chunk.1), "{kind:?} at {chunk:?}");
        if kind.grid().is_some() {
            assert_eq!(generator.locate_structure(kind, 0, 0), Some(chunk), "{kind:?}");
        }
    }
}

#[test]
fn mineshaft_chance_needs_distance_from_origin() {
    // nextInt(80) < max(|x|, |z|): no mineshaft can start in the origin chunk.
    assert!(!super::placement::is_mineshaft_chunk(20261003, (0, 0)));
}
