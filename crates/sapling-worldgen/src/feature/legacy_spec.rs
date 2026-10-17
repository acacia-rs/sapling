//! The shapes of the features BDS builds in code: plants, ice, coral and piles (`Legacy`), and the
//! tree, foliage and rock features (legacy_tree.rs, scripts/legacy_data.py).

use crate::block::Block;

#[derive(Debug, Clone, Copy)]
pub enum Legacy {
    DeadBush,
    Reeds,
    Cactus,
    Melon,
    Waterlily,
    /// `BlockPileFeature` with a pumpkin, `count` tries.
    Pumpkin { count: i32 },
    /// `BlockPileFeature` with a sweet berry bush, `count` tries.
    SweetBerries { count: i32 },
    Mushroom { block: Block, count: i32 },
    Flower { kind: FlowerKind, count: i32 },
    /// `firefly_bush_cluster_feature`, and `..._water_cluster_feature` beside water (land.rs).
    FireflyBush { water: bool },
    /// `legacy:ice_feature`: an iceberg in 1 of 8 chunks (iceberg.rs).
    Iceberg,
    /// `legacy:springs_feature`: water and lava springs in cave walls (springs.rs).
    Springs,
    /// `ice_patch_feature`, `ice_spike_feature` (ice_spike.rs).
    IcePatch,
    IceSpike,
    /// `pickle_feature` (aquatic.rs).
    SeaPickle,
    /// `bamboo_feature`, `podzol_area_feature` and `vines_feature` (jungle.rs).
    Bamboo,
    PodzolArea,
    Vines,
    /// `pale_moss_vegetation_feature` and `eyeblossom_feature` (pale.rs).
    PaleMossVegetation,
    Eyeblossom,
    /// `meadow_flower_feature` (flowers.rs).
    MeadowFlower,
    /// `grass_disc_feature` (disk.rs).
    GrassDisc,
    /// The warm ocean's `coral_crust_feature`, `coral_feature`, `coral_hang_feature` and
    /// `sea_anemone_feature` (coral/).
    CoralCrust,
    Coral,
    CoralHang,
    SeaAnemone,
    /// The village piles (pile.rs).
    Pile(PileBlock),
    /// The basalt deltas' and soul sand valleys' features (basalt.rs).
    BasaltColumns,
    BasaltPillar,
    Delta,
    /// `HugeFungusFeature` (fungus.rs): crimson, or warped without vines.
    HugeFungus { crimson: bool },
    /// The vine clusters (nether_vines.rs).
    WeepingVines,
    TwistingVines,
    /// `NetherFossilFeature` (nether_fossil.rs).
    NetherFossil,
}

/// The block of a village pile feature (`hay_pile_feature` and the others, vtable 14a9258d0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PileBlock {
    Hay,
    Melon,
    Pumpkin,
    Snow,
    Ice,
}

/// The flower table of a `LegacyFlowerFeature` (1438ef3b0's switch; its forest case is not ported).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowerKind {
    Overworld,
    Plains,
    Swamp,
    FlowerForest,
}

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
