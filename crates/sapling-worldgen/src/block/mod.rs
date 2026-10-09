//! The blocks the stages place, with their Bedrock names and states (table.rs), and the block
//! classes the stages test.

macro_rules! blocks {
    ($($variant:ident => $name:literal $states:literal,)*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[repr(u16)]
        pub enum Block {
            /// A block only structure templates place: an index into `template::palette()`. First, so
            /// the ranges below never hold it.
            Template(u16),
            $($variant,)*
        }

        impl Block {
            pub const ALL: &[Block] = &[$(Block::$variant,)*];

            /// The Bedrock name without the `minecraft:` prefix.
            pub fn name(self) -> &'static str {
                match self {
                    Block::Template(i) => $crate::structure::template::palette()[i as usize].0,
                    $(Block::$variant => $name,)*
                }
            }

            /// Non-default states as BDS saves them, `key=value` comma-separated.
            pub fn states(self) -> &'static str {
                match self {
                    Block::Template(i) => $crate::structure::template::palette()[i as usize].1,
                    $(Block::$variant => $states,)*
                }
            }
        }
    };
}

#[macro_use]
mod table;
#[macro_use]
mod cave_table;
#[macro_use]
mod structure_table;
#[macro_use]
mod multiface_table;
mod plant_table;

use std::collections::HashMap;
use std::sync::LazyLock;

pub use plant_table::Block;

use crate::structure::template::{COLLIDES, palette};

/// Kelp by `kelp_age`.
pub const KELP: [Block; 26] = [
    Block::Kelp0,
    Block::Kelp1,
    Block::Kelp2,
    Block::Kelp3,
    Block::Kelp4,
    Block::Kelp5,
    Block::Kelp6,
    Block::Kelp7,
    Block::Kelp8,
    Block::Kelp9,
    Block::Kelp10,
    Block::Kelp11,
    Block::Kelp12,
    Block::Kelp13,
    Block::Kelp14,
    Block::Kelp15,
    Block::Kelp16,
    Block::Kelp17,
    Block::Kelp18,
    Block::Kelp19,
    Block::Kelp20,
    Block::Kelp21,
    Block::Kelp22,
    Block::Kelp23,
    Block::Kelp24,
    Block::Kelp25,
];

impl Block {
    pub const CHESTS: [Block; 4] = [Block::ChestSouth, Block::ChestWest, Block::ChestNorth, Block::ChestEast];

    /// Every `Template` block, by palette index.
    pub fn templates() -> impl Iterator<Item = Block> {
        (0..palette().len() as u16).map(Block::Template)
    }

    /// The block's position in `ALL`; `Template` blocks have none.
    pub fn index(self) -> usize {
        debug_assert!(!matches!(self, Block::Template(_)));
        // SAFETY: a `repr(u16)` enum starts with its u16 discriminant; `Template` is 0.
        unsafe { *(&self as *const Block).cast::<u16>() as usize - 1 }
    }

    /// Water or lava of any depth.
    pub fn is_fluid(self) -> bool {
        (Block::Water..=Block::Lava15).contains(&self)
    }

    pub fn is_lava(self) -> bool {
        (Block::Lava..=Block::Lava15).contains(&self)
    }

    /// The `liquid_depth` of water or lava.
    pub fn liquid_depth(self) -> Option<u8> {
        let base = if self.is_lava() { Block::Lava } else { Block::Water };
        self.is_fluid().then(|| (self.index() - base.index()) as u8)
    }

    /// Water or lava with `liquid_depth` `depth` (0..16).
    pub fn liquid(lava: bool, depth: u8) -> Block {
        let base = if lava { Block::Lava } else { Block::Water };
        Block::ALL[base.index() + depth as usize]
    }

    /// Plants placed by features: no collision, no light blocking, never terrain.
    pub fn is_plant(self) -> bool {
        self >= Block::ShortGrass && !self.is_wet_speleothem() || (Block::GlowLichen1..=Block::SculkVein63).contains(&self)
    }

    /// Pointed dripstone with water in the second layer.
    pub fn is_wet_speleothem(self) -> bool {
        self >= Block::PointedDripstoneWet
    }

    /// FacingID (down, up, north, south, west, east) to `multi_face_direction_bits`.
    pub const FACE_BITS: [u8; 6] = [1, 2, 16, 4, 8, 32];

    /// A multiface block as its family's first variant and its face bits.
    pub fn multiface(self) -> Option<(Block, u8)> {
        [Block::GlowLichen1, Block::SculkVein1]
            .into_iter()
            .find(|&first| (first..=Block::with_faces(first, 63)).contains(&self))
            .map(|first| (first, (self.index() - first.index() + 1) as u8))
    }

    /// The variant of the family of `first` with face bits `bits` (1..=63).
    pub fn with_faces(first: Block, bits: u8) -> Block {
        Block::ALL[first.index() + bits as usize - 1]
    }

    /// Plants that stand in water: BDS saves them with water in the second layer.
    pub fn is_waterlogged(self) -> bool {
        self >= Block::Seagrass || matches!(self, Block::Template(i) if palette()[i as usize].2)
    }

    pub fn is_kelp(self) -> bool {
        (Block::Kelp0..=Block::Kelp25).contains(&self)
    }

    /// `BlockType::mFalling` (+0x163 bit 0x10): blocks that fall when unsupported.
    pub fn falls(self) -> bool {
        let name = self.name();
        matches!(name, "sand" | "red_sand" | "gravel" | "suspicious_sand" | "suspicious_gravel" | "anvil" | "dragon_egg") || name.ends_with("concrete_powder")
    }

    /// Logs and leaves.
    pub fn is_wood(self) -> bool {
        (Block::OakLog..=Block::AzaleaLeavesFlowered).contains(&self)
    }

    pub fn is_leaves(self) -> bool {
        (Block::OakLeaves..=Block::AzaleaLeavesFlowered).contains(&self)
    }

    /// Blocks only structure pieces place (structure_table.rs).
    pub fn is_structure(self) -> bool {
        (Block::OakPlanks..=Block::EndPortal).contains(&self)
    }

    pub fn is_solid(self) -> bool {
        match self {
            Block::Template(i) => palette()[i as usize].3 & COLLIDES != 0,
            _ => self != Block::Air && !self.is_fluid() && !self.is_plant(),
        }
    }

    /// Solid for movement: everything but air, fluids, snow layers and the plants without collision.
    pub fn blocks_motion(self) -> bool {
        self.is_solid() && self != Block::SnowLayer || self.is_crop_block()
    }

    /// The feature blocks that are full blocks: they block motion and light.
    pub fn is_crop_block(self) -> bool {
        matches!(
            self,
            Block::Cactus | Block::MelonBlock | Block::PumpkinSouth | Block::PumpkinWest | Block::PumpkinNorth | Block::PumpkinEast | Block::MangroveRootsWet
        )
    }

    /// A waterlogged twin as the block a descriptor names (`BlockDescriptor` ignores the liquid).
    pub fn dry(self) -> Block {
        match self {
            Block::MangroveRootsWet => Block::MangroveRoots,
            block if block.is_wet_speleothem() => Block::ALL[block.index() - Block::PointedDripstoneWet.index() + Block::PointedDripstone.index()],
            block => block,
        }
    }

    /// The block written over water: waterloggable blocks keep it (`BlockSource::setBlock`).
    pub fn into_water(self) -> Block {
        match self {
            Block::MangroveRoots => Block::MangroveRootsWet,
            block if (Block::PointedDripstone..=Block::PointedDripstoneMergeHanging).contains(&block) => {
                Block::ALL[block.index() - Block::PointedDripstone.index() + Block::PointedDripstoneWet.index()]
            }
            block => block,
        }
    }

    /// A down-facing bud turned to `face` (FacingID 0..6).
    pub fn faced(self, face: usize) -> Block {
        Block::ALL[self.index() + face]
    }

    /// A speleothem (`PointedDripstone` or `SulfurSpike`) of thickness 0..5 (tip, frustum, middle,
    /// base, merge), hanging or standing.
    pub fn speleothem(self, thickness: usize, hanging: bool) -> Block {
        Block::ALL[self.index() + thickness * 2 + usize::from(hanging)]
    }

    pub fn sea_pickle(count: i32) -> Block {
        Block::ALL[Block::SeaPickle0.index() + count as usize]
    }

    pub fn is_sea_pickle(self) -> bool {
        (Block::SeaPickle0..=Block::SeaPickle3).contains(&self)
    }

    /// The block for a `minecraft:` name, without states.
    pub fn from_name(name: &str) -> Option<Block> {
        let name = name.strip_prefix("minecraft:").unwrap_or(name);
        Block::ALL.iter().copied().find(|block| block.name() == name)
    }

    /// The block with exactly this name and these states, if the table has it.
    pub fn with_states(name: &str, states: &str) -> Option<Block> {
        static BY_NAME: LazyLock<HashMap<&str, Vec<Block>>> = LazyLock::new(|| {
            let mut by_name: HashMap<&str, Vec<Block>> = HashMap::new();
            Block::ALL.iter().for_each(|&b| by_name.entry(b.name()).or_default().push(b));
            by_name
        });
        BY_NAME.get(name)?.iter().copied().find(|b| b.states() == states)
    }
}
