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

mod table;

pub use table::Block;

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
    pub fn is_fluid(self) -> bool {
        matches!(self, Block::Water | Block::Lava)
    }

    /// Plants placed by features: no collision, no light blocking, never terrain.
    pub fn is_plant(self) -> bool {
        self >= Block::ShortGrass
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
        !matches!(self, Block::Air | Block::Water | Block::Lava) && !self.is_plant()
    }

    /// Solid for movement: everything but air, fluids, snow layers and the plants without collision.
    pub fn blocks_motion(self) -> bool {
        self.is_solid() && self != Block::SnowLayer || self.is_crop_block()
    }

    /// The feature blocks that are full blocks: they block motion and light.
    pub fn is_crop_block(self) -> bool {
        matches!(self, Block::Cactus | Block::MelonBlock | Block::PumpkinSouth | Block::PumpkinWest | Block::PumpkinNorth | Block::PumpkinEast)
    }

    /// The block for a `minecraft:` name, without states.
    pub fn from_name(name: &str) -> Option<Block> {
        let name = name.strip_prefix("minecraft:").unwrap_or(name);
        Block::ALL.iter().copied().find(|block| block.name() == name)
    }
}
