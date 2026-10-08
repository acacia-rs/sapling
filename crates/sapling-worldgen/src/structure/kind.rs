//! The overworld structure kinds, their placement grids and the biome lists their start checks
//! use (feature constructors called from 148810880; v2 sets from the pack JSON and BDS's own).

use super::placement::{Grid, Spread};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StructureKind {
    Village,
    Stronghold,
    /// Desert and jungle pyramids, swamp huts and igloos (`RandomScatteredLargeFeature`).
    Temple,
    Mineshaft,
    Mansion,
    BuriedTreasure,
    Outpost,
    Monument,
    OceanRuins,
    Shipwreck,
    RuinedPortal,
    AncientCity,
    /// Jigsaw ("v2") structure sets, placed by Java's LCG.
    TrialChambers,
    TrailRuins,
    /// The `abandoned_camp` set: one of 18 biome variants by weight.
    AbandonedCamp,
}

impl StructureKind {
    pub const ALL: [StructureKind; 15] = [
        Self::Village,
        Self::Stronghold,
        Self::Temple,
        Self::Mineshaft,
        Self::Mansion,
        Self::BuriedTreasure,
        Self::Outpost,
        Self::Monument,
        Self::OceanRuins,
        Self::Shipwreck,
        Self::RuinedPortal,
        Self::AncientCity,
        Self::TrialChambers,
        Self::TrailRuins,
        Self::AbandonedCamp,
    ];

    /// The name `/locate structure` takes.
    pub fn name(self) -> &'static str {
        match self {
            Self::Village => "village",
            Self::Stronghold => "stronghold",
            Self::Temple => "temple",
            Self::Mineshaft => "mineshaft",
            Self::Mansion => "mansion",
            Self::BuriedTreasure => "buried_treasure",
            Self::Outpost => "pillager_outpost",
            Self::Monument => "monument",
            Self::OceanRuins => "ruins",
            Self::Shipwreck => "shipwreck",
            Self::RuinedPortal => "ruined_portal",
            Self::AncientCity => "ancient_city",
            Self::TrialChambers => "trial_chambers",
            Self::TrailRuins => "trail_ruins",
            Self::AbandonedCamp => "abandoned_camp",
        }
    }

    /// The legacy MT grid a kind's candidates come from; strongholds and mineshafts have none.
    pub(super) fn grid(self) -> Option<Grid> {
        Some(match self {
            Self::Village => Grid::new(34, 8, 10_387_312, Spread::Triangular),
            Self::Temple => Grid::new(32, 8, 14_357_617, Spread::Linear),
            Self::Mansion => Grid::new(80, 20, 10_387_319, Spread::Triangular),
            Self::BuriedTreasure => Grid::new(4, 2, 16_842_397, Spread::Triangular),
            Self::Outpost => Grid::new(80, 24, 165_745_296, Spread::Triangular),
            Self::Monument => MONUMENT,
            Self::OceanRuins => Grid::new(20, 8, 14_357_621, Spread::Linear),
            Self::Shipwreck => Grid::new(24, 4, 165_745_295, Spread::Linear),
            Self::RuinedPortal => Grid::new(40, 15, 40_552_231, Spread::Linear),
            Self::AncientCity => Grid::new(24, 8, 20_083_232, Spread::Triangular),
            _ => return None,
        })
    }

    /// The v2 structure set's Java-LCG grid (`RandomSpreadPlacement` 144cfea60).
    pub(super) fn java_grid(self) -> Option<Grid> {
        Some(match self {
            Self::TrialChambers => Grid::new(34, 12, 94_251_327, Spread::Linear),
            Self::TrailRuins => Grid::new(34, 8, 83_469_867, Spread::Linear),
            Self::AbandonedCamp => Grid::new(37, 8, 91_231_127, Spread::Linear),
            _ => return None,
        })
    }
}

pub(super) const MONUMENT: Grid = Grid::new(32, 5, 10_387_313, Spread::Triangular);

/// Biomes with a `minecraft:village_type` component.
pub(super) const VILLAGE_BIOMES: &[u32] = &[1, 2, 5, 12, 19, 30, 31, 35, 129, 186];
pub(super) const TEMPLE_BIOMES: &[u32] = &[2, 17, 21, 22, 6, 134, 12, 30, 184];
pub(super) const MANSION_BIOMES: &[u32] = &[193, 29, 157, 188, 187, 194];
pub(super) const BURIED_TREASURE_BIOMES: &[u32] = &[16, 26, 25, 15];
pub(super) const OUTPOST_BIOMES: &[u32] = &[1, 129, 35, 12, 5, 19, 30, 31, 2, 186, 183, 182, 189, 184, 192, 185];
pub(super) const DEEP_OCEANS: &[u32] = &[24, 45, 43, 41, 47];
pub(super) const MONUMENT_SURROUNDINGS: &[u32] = &[0, 24, 44, 45, 46, 47, 42, 43, 40, 41, 11, 7];
pub(super) const OCEAN_RUINS_BIOMES: &[u32] = &[0, 24, 45, 47, 43, 44, 46, 42, 40, 41];
pub(super) const SHIPWRECK_BIOMES: &[u32] = &[16, 26, 0, 44, 42, 46, 15, 24, 45, 43, 47, 40];
/// Mushroom shore, beach, snowy beach: shipwrecks there need only 10 blocks of the same biome.
pub(super) const BEACHES: &[u32] = &[15, 16, 26];
pub(super) const DEEP_DARK: &[u32] = &[190];
/// Biomes tagged `has_structure_trail_ruins`.
pub(super) const TRAIL_RUINS_BIOMES: &[u32] = &[155, 30, 21, 32, 160, 5];
