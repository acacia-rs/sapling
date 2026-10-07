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
}

impl Block {
    pub fn is_fluid(self) -> bool {
        matches!(self, Block::Water | Block::Lava)
    }

    pub fn is_solid(self) -> bool {
        !matches!(self, Block::Air | Block::Water | Block::Lava)
    }

    /// The block for a `minecraft:` name, without states.
    pub fn from_name(name: &str) -> Option<Block> {
        let name = name.strip_prefix("minecraft:").unwrap_or(name);
        Block::ALL.iter().copied().find(|block| block.name() == name)
    }
}
