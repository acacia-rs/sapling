//! The shapes of the hardcoded tree, foliage and rock features (legacy_tree.rs, scripts/legacy_data.py).

use crate::block::Block;

#[derive(Debug)]
pub enum LegacyTree {
    /// `LegacyTreeFeature`: a count drawn from `count`, then per tree a feature `choice` picks
    /// from `features`, at a random column of the chunk's heightmap.
    Trees { count: f32, choice: TreeChoice, features: &'static [usize], grass: usize },
    /// `LegacyForestFoliageFeature`: double plants (lilac, rose bush, peony) at random heights.
    ForestFoliage { flower_forest: bool },
    /// `LegacySwampFoliageFeature`: seagrass tries, then swamp oaks or a huge mushroom.
    SwampFoliage { seagrass: usize, tree: usize, huge_mushroom: usize, grass: usize },
    /// `LegacyForestRockFeature`: up to two mossy cobblestone blobs.
    ForestRock,
    /// `LeafLitterFeature` (also pink petals and wildflowers): one flower bed of a random size and
    /// facing; `bed` is its growth-0 south-facing block.
    FlowerBed { bed: Block },
    /// `HugeMushroomFeature` of a random type (huge_mushroom.rs).
    HugeMushroom,
}

/// The per-type tree pickers of `LegacyTreeFeature` (its constructor's lambdas), each drawing
/// its own sequence; the doc names the index each outcome picks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeChoice {
    /// Always 0.
    First,
    /// `nextInt(3) == 0` → 0, else 1.
    OneInThree,
    /// `nextInt(5) > 0` → 0, else 1.
    FourInFive,
    /// `nextInt(5) == 0` → 0, else `nextBoolean` → 1 or 0.
    BirchMutated,
    /// `nextInt(3) > 0` → 0, else `nextInt(10) == 0` → 1, else 2.
    ExtremeHills,
    /// `nextInt(5) == 0` → 0, else `nextInt(10) == 0` → 1, else 2.
    Forest,
    /// `nextBoolean` → 0, else 1.
    Meadow,
    /// `nextInt(10) == 0` → 0, else `nextInt(2) == 0` → 1, else `nextInt(3) == 0` → 2, else 3.
    Jungle,
    /// `nextInt(10) == 0` → 0, else `nextInt(2) == 0` → 1, else 2.
    JungleEdge,
    /// `nextInt(20) == 0` → 0, else `nextInt(20) < 3` → 1, else `nextInt(10) < 7` → 2, else 3.
    BambooJungle,
    /// `nextInt(3) == 0` → (`nextInt(13) == 0` → 0, else 1), else `nextInt(3) == 0` → 2, else 3.
    MegaTaiga,
    /// `nextInt(3) == 0` → 0, else `nextInt(3) == 0` → 1, else 2.
    RedwoodMutated,
}
