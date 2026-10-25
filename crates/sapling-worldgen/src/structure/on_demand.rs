//! Structures started on demand (`/place structure`, `/place jigsaw`): a kind's start forced into
//! a chunk, its placement grid and biome filter skipped, as the structure stage would draw it.

use std::sync::Arc;

use super::jigsaw::{Piece, StartHeight, V2Start, data};
use super::probe::{Probe, RawHeights};
use super::{BoundingBox, Reaching, SEA_LEVEL, StructureKind, Structures, igloo};
use crate::density::Density;
use crate::random::JavaRandom;

/// Why a structure could not be placed on demand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceFailure {
    /// No such structure or jigsaw pool.
    Unknown,
    /// A kind this cannot start on demand.
    Unsupported,
    /// The start's own checks failed there (no room, wrong height, no variant for the biome...).
    Failed,
}

/// `/place jigsaw`'s reach from the start piece (Java's `maxDistanceFromCenter`).
const JIGSAW_REACH: i32 = 128;

/// A forced start: what the post-processes it reaches place, and the box of chunks they are
/// (None: the start chunk alone, which places the structure whole).
pub struct Forced {
    pub reaching: Reaching,
    pub bb: Option<BoundingBox>,
}

fn union(boxes: impl Iterator<Item = BoundingBox>) -> Option<BoundingBox> {
    boxes.reduce(|a, b| a.union(&b))
}

impl Forced {
    fn spanning(reaching: Reaching, bb: Option<BoundingBox>) -> Result<Self, PlaceFailure> {
        bb.map(|bb| Self { reaching, bb: Some(bb) }).ok_or(PlaceFailure::Failed)
    }

    fn whole(reaching: Reaching) -> Result<Self, PlaceFailure> {
        Ok(Self { reaching, bb: None })
    }

    fn jigsaw(pieces: Arc<Vec<Piece>>) -> Result<Self, PlaceFailure> {
        let bb = union(pieces.iter().map(|p| p.bb));
        Self::spanning(Reaching { v2: vec![pieces], ..Default::default() }, bb)
    }
}

impl Structures {
    /// `kind` started in `start` whatever the grid and biome filter say; the variant a biome picks
    /// (village type, temple kind, warm ruins) still comes from the start's biome.
    pub(crate) fn forced(&self, density: &Density, kind: StructureKind, start: (i32, i32)) -> Result<Forced, PlaceFailure> {
        let probe = Probe { density };
        let none = Reaching::default;
        match kind {
            StructureKind::Village => {
                let village = self.village(density, start);
                let bb = union(village.pieces.iter().map(|p| p.bb));
                Forced::spanning(Reaching { villages: vec![village], ..none() }, bb)
            }
            StructureKind::Stronghold => {
                let stronghold = self.stronghold(start);
                let bb = Some(stronghold.bb);
                Forced::spanning(Reaching { strongholds: vec![stronghold], ..none() }, bb)
            }
            StructureKind::Temple if probe.min_height(start) < SEA_LEVEL => Err(PlaceFailure::Failed),
            StructureKind::Temple if igloo::IGLOO_BIOMES.contains(&Self::centre_biome(density, start)) => Forced::whole(Reaching { igloos: vec![start], ..none() }),
            StructureKind::Temple => {
                let temple = self.scattered(density, start).ok_or(PlaceFailure::Failed)?;
                let bb = Some(temple.bb);
                Forced::spanning(Reaching { temples: vec![temple], ..none() }, bb)
            }
            StructureKind::Mineshaft => {
                let mineshaft = self.mineshaft(density, start);
                let bb = Some(mineshaft.bb);
                Forced::spanning(Reaching { mineshafts: vec![mineshaft], ..none() }, bb)
            }
            StructureKind::Mansion => {
                let mansion = self.mansion(density, start);
                let bb = Some(mansion.bb);
                Forced::spanning(Reaching { mansions: vec![mansion], ..none() }, bb)
            }
            StructureKind::BuriedTreasure => Forced::whole(Reaching { treasure: true, ..none() }),
            StructureKind::Outpost => {
                let outpost = self.outpost(start);
                let bb = union(outpost.pieces.iter().map(|p| p.bb));
                Forced::spanning(Reaching { outposts: vec![outpost], ..none() }, bb)
            }
            StructureKind::Monument => {
                let monument = self.monument(start);
                let bb = Some(monument.building.bb);
                Forced::spanning(Reaching { monuments: vec![monument], ..none() }, bb)
            }
            StructureKind::OceanRuins => {
                let ruin = self.ocean_ruin(density, start);
                let bb = union(ruin.pieces.iter().map(|p| p.bb));
                Forced::spanning(Reaching { ruins: vec![ruin], ..none() }, bb)
            }
            StructureKind::Shipwreck => Forced::whole(Reaching { shipwrecks: vec![(start, Self::beached(density, start))], ..none() }),
            StructureKind::RuinedPortal => Forced::whole(Reaching { portals: vec![start], ..none() }),
            StructureKind::AncientCity => {
                let city = self.ancient_city(start);
                let bb = union(city.pieces.iter().map(|p| p.bb));
                Forced::spanning(Reaching { cities: vec![city], ..none() }, bb)
            }
            StructureKind::TrialChambers | StructureKind::TrailRuins | StructureKind::AbandonedCamp => {
                let pieces = self.forced_v2(density, kind, start).ok_or(PlaceFailure::Failed)?;
                Forced::jigsaw(pieces)
            }
        }
    }

    /// `/place jigsaw <pool> <target> <max_depth> <at>`: the v2 placer from `pool`'s start element,
    /// its jigsaw named `target` at `at`, drawing from `at`'s chunk's large-feature random.
    pub(crate) fn forced_jigsaw(&self, density: &Density, pool: &str, target: &str, max_depth: u32, at: [i32; 3]) -> Result<Forced, PlaceFailure> {
        let pool = data().pool(pool).ok_or(PlaceFailure::Unknown)?;
        let target = if target.contains(':') { target.to_string() } else { format!("minecraft:{target}") };
        let start = V2Start {
            pool: &pool.name,
            max_depth: max_depth.min(i32::MAX as u32) as i32,
            horizontal: JIGSAW_REACH,
            vertical: JIGSAW_REACH,
            height: StartHeight::Constant(0),
            project: false,
            padding: 0,
            aliases: None,
        };
        let mut random = JavaRandom::large_feature(self.seed, at[0] >> 4, at[2] >> 4);
        let (piece, centre) = start.named_start(&target, (at[0], at[1], at[2]), &mut random).ok_or(PlaceFailure::Failed)?;
        let raw = RawHeights::new(density);
        Forced::jigsaw(Arc::new(start.grow(piece, centre, Vec::new(), &mut random, &|x, z| raw.at(x, z))))
    }
}
