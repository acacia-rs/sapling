//! The blocks the stages place, with their Bedrock names and states (table.rs), and the block
//! classes the stages test.

macro_rules! blocks {
    ($($variant:ident => $name:literal $states:literal,)*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum Block {
            $($variant,)*
        }

        impl Block {
            pub const ALL: &[Block] = &[$(Block::$variant,)*];

            /// The Bedrock name without the `minecraft:` prefix.
            pub fn name(self) -> &'static str {
                match self {
                    $(Block::$variant => $name,)*
                }
            }

            /// Non-default states as BDS saves them, `key=value` comma-separated.
            pub fn states(self) -> &'static str {
                match self {
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
        self.is_fluid().then(|| (self as usize - base as usize) as u8)
    }

    /// Water or lava with `liquid_depth` `depth` (0..16).
    pub fn liquid(lava: bool, depth: u8) -> Block {
        let base = if lava { Block::Lava } else { Block::Water };
        Block::ALL[base as usize + depth as usize]
    }

    /// Plants placed by features: no collision, no light blocking, never terrain.
    pub fn is_plant(self) -> bool {
        self >= Block::ShortGrass || (Block::GlowLichen1..=Block::SculkVein63).contains(&self)
    }

    /// FacingID (down, up, north, south, west, east) to `multi_face_direction_bits`.
    pub const FACE_BITS: [u8; 6] = [1, 2, 16, 4, 8, 32];

    /// A multiface block as its family's first variant and its face bits.
    pub fn multiface(self) -> Option<(Block, u8)> {
        [Block::GlowLichen1, Block::SculkVein1]
            .into_iter()
            .find(|&first| (first as usize..first as usize + 63).contains(&(self as usize)))
            .map(|first| (first, (self as usize - first as usize + 1) as u8))
    }

    /// The variant of the family of `first` with face bits `bits` (1..=63).
    pub fn with_faces(first: Block, bits: u8) -> Block {
        Block::ALL[first as usize + bits as usize - 1]
    }

    /// Plants that stand in water: BDS saves them with water in the second layer.
    pub fn is_waterlogged(self) -> bool {
        self >= Block::Seagrass
    }

    pub fn is_kelp(self) -> bool {
        self >= Block::Kelp0
    }

    /// Logs and leaves.
    pub fn is_wood(self) -> bool {
        (Block::OakLog..=Block::AzaleaLeavesFlowered).contains(&self)
    }

    pub fn is_leaves(self) -> bool {
        (Block::OakLeaves..=Block::AzaleaLeavesFlowered).contains(&self)
    }

    pub fn is_solid(self) -> bool {
        self != Block::Air && !self.is_fluid() && !self.is_plant()
    }

    /// Solid for movement: everything but air, fluids, snow layers and the plants without collision.
    pub fn blocks_motion(self) -> bool {
        self.is_solid() && self != Block::SnowLayer || self.is_crop_block()
    }

    /// The feature blocks that are full blocks: they block motion and light.
    pub fn is_crop_block(self) -> bool {
        matches!(self, Block::Cactus | Block::MelonBlock | Block::PumpkinSouth | Block::PumpkinWest | Block::PumpkinNorth | Block::PumpkinEast)
    }

    /// A down-facing bud turned to `face` (FacingID 0..6).
    pub fn faced(self, face: usize) -> Block {
        let index = Block::ALL.iter().position(|&block| block == self).expect("in ALL");
        Block::ALL[index + face]
    }

    /// A speleothem (`PointedDripstone` or `SulfurSpike`) of thickness 0..5 (tip, frustum, middle,
    /// base, merge), hanging or standing.
    pub fn speleothem(self, thickness: usize, hanging: bool) -> Block {
        Block::ALL[self as usize + thickness * 2 + usize::from(hanging)]
    }

    pub fn sea_pickle(count: i32) -> Block {
        Block::ALL[Block::SeaPickle0 as usize + count as usize]
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
