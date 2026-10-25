//! The structure starts whose pieces one chunk's post-process places, gathered before it runs: a
//! chunk none reaches has no post-process to order.

use std::sync::Arc;

use super::jigsaw::Piece;
use super::mineshaft::Mineshaft;
use super::monument::Monument;
use super::scattered::Scattered;
use super::stronghold::Stronghold;
use super::{BoundingBox, StructureKind, Structures, ancient_city, mansion, ocean_ruin, outpost, village};
use crate::density::Density;

/// Pieces stay within 80 blocks of the start's room plus a corridor, about 7 chunks.
const MINESHAFT_REACH: i32 = 7;
/// Stronghold doors stay within 112 blocks of the start; pieces and the portal room fallback reach further.
pub(super) const STRONGHOLD_REACH: i32 = 9;
/// A desert pyramid reaches one chunk past its start in x and z.
const TEMPLE_REACH: i32 = 1;
/// A monument's building spans its start chunk and two chunks either way.
const MONUMENT_REACH: i32 = 2;

#[derive(Default)]
pub struct Reaching {
    pub(super) strongholds: Vec<Arc<Stronghold>>,
    pub(super) temples: Vec<Scattered>,
    /// By start, ascending x then z; the post-process visits them in the start map's order.
    pub(super) mineshafts: Vec<Arc<Mineshaft>>,
    pub(super) treasure: bool,
    pub(super) monuments: Vec<Arc<Monument>>,
    /// Igloos, shipwrecks (with whether each is beached) and ruined portals starting in the chunk:
    /// the start chunk places them whole.
    pub(super) igloos: Vec<(i32, i32)>,
    pub(super) shipwrecks: Vec<((i32, i32), bool)>,
    pub(super) portals: Vec<(i32, i32)>,
    pub(super) ruins: Vec<Arc<ocean_ruin::OceanRuin>>,
    pub(super) outposts: Vec<Arc<outpost::Outpost>>,
    pub(super) cities: Vec<Arc<ancient_city::AncientCity>>,
    pub(super) villages: Vec<Arc<village::Village>>,
    pub(super) v2: Vec<Arc<Vec<Piece>>>,
    pub(super) mansions: Vec<Arc<mansion::Mansion>>,
}

impl Reaching {
    fn is_empty(&self) -> bool {
        self.strongholds.is_empty()
            && self.temples.is_empty()
            && self.mineshafts.is_empty()
            && !self.treasure
            && self.monuments.is_empty()
            && self.igloos.is_empty()
            && self.shipwrecks.is_empty()
            && self.portals.is_empty()
            && self.ruins.is_empty()
            && self.outposts.is_empty()
            && self.cities.is_empty()
            && self.villages.is_empty()
            && self.v2.is_empty()
            && self.mansions.is_empty()
    }
}

impl Structures {
    /// What `chunk`'s post-process places; None when no start reaches it.
    pub fn reaching(&self, density: &Density, chunk: (i32, i32)) -> Option<Reaching> {
        let column = BoundingBox::chunk(chunk.0, chunk.1);
        let near = |kind, reach| self.starts_near(density, kind, chunk, reach).into_iter();
        let mut igloos = self.igloos_near(density, chunk);
        igloos.retain(|start| *start == chunk);
        let mut shipwrecks = self.shipwrecks_near(density, chunk);
        shipwrecks.retain(|(start, _)| *start == chunk);
        let mut portals = self.portals_near(density, chunk);
        portals.retain(|start| *start == chunk);
        let reaching = Reaching {
            strongholds: near(StructureKind::Stronghold, STRONGHOLD_REACH).map(|start| self.stronghold(start)).filter(|s| s.bb.intersects(&column)).collect(),
            temples: near(StructureKind::Temple, TEMPLE_REACH).filter_map(|start| self.scattered(density, start)).filter(|t| t.bb.intersects(&column)).collect(),
            mineshafts: near(StructureKind::Mineshaft, MINESHAFT_REACH).map(|start| self.mineshaft(density, start)).filter(|m| m.bb.intersects(&column)).collect(),
            treasure: self.is_start(density, StructureKind::BuriedTreasure, chunk),
            monuments: near(StructureKind::Monument, MONUMENT_REACH).map(|start| self.monument(start)).filter(|m| m.building.bb.intersects(&column)).collect(),
            igloos,
            shipwrecks,
            portals,
            ruins: self.ocean_ruins_near(density, chunk),
            outposts: self.outposts_near(density, chunk),
            cities: self.ancient_cities_near(density, chunk),
            villages: self.villages_near(density, chunk),
            v2: self.v2_near(density, chunk),
            mansions: self.mansions_near(density, chunk),
        };
        (!reaching.is_empty()).then_some(reaching)
    }
}
