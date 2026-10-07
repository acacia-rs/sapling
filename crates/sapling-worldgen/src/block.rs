//! The blocks the terrain stages place, with their Bedrock names and states.

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

blocks! {
    Air => "air" "",
    Water => "water" "liquid_depth=0",
    Lava => "lava" "liquid_depth=0",
    Bedrock => "bedrock" "infiniburn_bit=0",
    Stone => "stone" "",
    Deepslate => "deepslate" "pillar_axis=y",
    Granite => "granite" "",
    Tuff => "tuff" "",
    CopperOre => "copper_ore" "",
    DeepslateCopperOre => "deepslate_copper_ore" "",
    RawCopperBlock => "raw_copper_block" "",
    IronOre => "iron_ore" "",
    DeepslateIronOre => "deepslate_iron_ore" "",
    RawIronBlock => "raw_iron_block" "",
    GrassBlock => "grass_block" "",
    Dirt => "dirt" "",
    CoarseDirt => "coarse_dirt" "",
    Podzol => "podzol" "",
    Mycelium => "mycelium" "",
    Mud => "mud" "",
    Sand => "sand" "",
    RedSand => "red_sand" "",
    Sandstone => "sandstone" "",
    RedSandstone => "red_sandstone" "",
    Gravel => "gravel" "",
    Calcite => "calcite" "",
    Snow => "snow" "",
    SnowLayer => "snow_layer" "covered_bit=0,height=0",
    PowderSnow => "powder_snow" "",
    Ice => "ice" "",
    PackedIce => "packed_ice" "",
    Terracotta => "hardened_clay" "",
    WhiteTerracotta => "white_terracotta" "",
    OrangeTerracotta => "orange_terracotta" "",
    YellowTerracotta => "yellow_terracotta" "",
    BrownTerracotta => "brown_terracotta" "",
    RedTerracotta => "red_terracotta" "",
    LightGrayTerracotta => "light_gray_terracotta" "",
    Andesite => "andesite" "",
    Diorite => "diorite" "",
    Clay => "clay" "",
    CoalOre => "coal_ore" "",
    DeepslateCoalOre => "deepslate_coal_ore" "",
    GoldOre => "gold_ore" "",
    DeepslateGoldOre => "deepslate_gold_ore" "",
    RedstoneOre => "redstone_ore" "",
    DeepslateRedstoneOre => "deepslate_redstone_ore" "",
    LapisOre => "lapis_ore" "",
    DeepslateLapisOre => "deepslate_lapis_ore" "",
    DiamondOre => "diamond_ore" "",
    DeepslateDiamondOre => "deepslate_diamond_ore" "",
    EmeraldOre => "emerald_ore" "",
    DeepslateEmeraldOre => "deepslate_emerald_ore" "",
    InfestedStone => "infested_stone" "",
    InfestedDeepslate => "infested_deepslate" "pillar_axis=y",
    // Plants from here on, waterlogged ones from Seagrass on (`is_plant`, `is_waterlogged`).
    ShortGrass => "short_grass" "",
    Fern => "fern" "",
    TallGrass => "tall_grass" "upper_block_bit=0",
    TallGrassTop => "tall_grass" "upper_block_bit=1",
    LargeFern => "large_fern" "upper_block_bit=0",
    LargeFernTop => "large_fern" "upper_block_bit=1",
    Sunflower => "sunflower" "upper_block_bit=0",
    SunflowerTop => "sunflower" "upper_block_bit=1",
    Bush => "bush" "",
    ShortDryGrass => "short_dry_grass" "",
    TallDryGrass => "tall_dry_grass" "",
    Seagrass => "seagrass" "sea_grass_type=default",
    SeagrassBottom => "seagrass" "sea_grass_type=double_bot",
    SeagrassTop => "seagrass" "sea_grass_type=double_top",
    Kelp0 => "kelp" "kelp_age=0",
    Kelp1 => "kelp" "kelp_age=1",
    Kelp2 => "kelp" "kelp_age=2",
    Kelp3 => "kelp" "kelp_age=3",
    Kelp4 => "kelp" "kelp_age=4",
    Kelp5 => "kelp" "kelp_age=5",
    Kelp6 => "kelp" "kelp_age=6",
    Kelp7 => "kelp" "kelp_age=7",
    Kelp8 => "kelp" "kelp_age=8",
    Kelp9 => "kelp" "kelp_age=9",
    Kelp10 => "kelp" "kelp_age=10",
    Kelp11 => "kelp" "kelp_age=11",
    Kelp12 => "kelp" "kelp_age=12",
    Kelp13 => "kelp" "kelp_age=13",
    Kelp14 => "kelp" "kelp_age=14",
    Kelp15 => "kelp" "kelp_age=15",
    Kelp16 => "kelp" "kelp_age=16",
    Kelp17 => "kelp" "kelp_age=17",
    Kelp18 => "kelp" "kelp_age=18",
    Kelp19 => "kelp" "kelp_age=19",
    Kelp20 => "kelp" "kelp_age=20",
    Kelp21 => "kelp" "kelp_age=21",
    Kelp22 => "kelp" "kelp_age=22",
    Kelp23 => "kelp" "kelp_age=23",
    Kelp24 => "kelp" "kelp_age=24",
    Kelp25 => "kelp" "kelp_age=25",
}

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

    pub fn is_solid(self) -> bool {
        !matches!(self, Block::Air | Block::Water | Block::Lava) && !self.is_plant()
    }

    /// Solid for movement: everything but air, fluids, plants and snow layers.
    pub fn blocks_motion(self) -> bool {
        self.is_solid() && self != Block::SnowLayer
    }

    /// The block for a `minecraft:` name, without states.
    pub fn from_name(name: &str) -> Option<Block> {
        let name = name.strip_prefix("minecraft:").unwrap_or(name);
        Block::ALL.iter().copied().find(|block| block.name() == name)
    }
}
