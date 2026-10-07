//! The blocks the terrain stages place, with their Bedrock names and states.

use crate::structure::template::{COLLIDES, TEMPLATE_BLOCKS};

macro_rules! blocks {
    ($($variant:ident => $name:literal $states:literal,)*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum Block {
            /// A block only structure templates place: an index into `TEMPLATE_BLOCKS`. First, so
            /// the plant ranges below never hold it.
            Template(u16),
            $($variant,)*
        }

        impl Block {
            pub const ALL: &[Block] = &[$(Block::$variant,)*];

            /// The Bedrock name without the `minecraft:` prefix.
            pub fn name(self) -> &'static str {
                match self {
                    Block::Template(i) => TEMPLATE_BLOCKS[i as usize].0,
                    $(Block::$variant => $name,)*
                }
            }

            /// Non-default states as BDS saves them, `key=value` comma-separated.
            pub fn states(self) -> &'static str {
                match self {
                    Block::Template(i) => TEMPLATE_BLOCKS[i as usize].1,
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
    // Structure blocks; fences by connection bits east 1, north 2, south 4, west 8.
    OakPlanks => "oak_planks" "",
    DarkOakPlanks => "dark_oak_planks" "",
    OakLog => "oak_log" "pillar_axis=y",
    DarkOakLog => "dark_oak_log" "pillar_axis=y",
    IronChain => "iron_chain" "pillar_axis=y",
    Web => "web" "",
    MobSpawner => "mob_spawner" "",
    Chest => "chest" "minecraft:cardinal_direction=north",
    RailNorthSouth => "rail" "rail_direction=0",
    RailEastWest => "rail" "rail_direction=1",
    TorchWest => "torch" "torch_facing_direction=west",
    TorchEast => "torch" "torch_facing_direction=east",
    TorchNorth => "torch" "torch_facing_direction=north",
    TorchSouth => "torch" "torch_facing_direction=south",
    OakFence0 => "oak_fence" "minecraft:connection_east=0,minecraft:connection_north=0,minecraft:connection_south=0,minecraft:connection_west=0",
    OakFence1 => "oak_fence" "minecraft:connection_east=1,minecraft:connection_north=0,minecraft:connection_south=0,minecraft:connection_west=0",
    OakFence2 => "oak_fence" "minecraft:connection_east=0,minecraft:connection_north=1,minecraft:connection_south=0,minecraft:connection_west=0",
    OakFence3 => "oak_fence" "minecraft:connection_east=1,minecraft:connection_north=1,minecraft:connection_south=0,minecraft:connection_west=0",
    OakFence4 => "oak_fence" "minecraft:connection_east=0,minecraft:connection_north=0,minecraft:connection_south=1,minecraft:connection_west=0",
    OakFence5 => "oak_fence" "minecraft:connection_east=1,minecraft:connection_north=0,minecraft:connection_south=1,minecraft:connection_west=0",
    OakFence6 => "oak_fence" "minecraft:connection_east=0,minecraft:connection_north=1,minecraft:connection_south=1,minecraft:connection_west=0",
    OakFence7 => "oak_fence" "minecraft:connection_east=1,minecraft:connection_north=1,minecraft:connection_south=1,minecraft:connection_west=0",
    OakFence8 => "oak_fence" "minecraft:connection_east=0,minecraft:connection_north=0,minecraft:connection_south=0,minecraft:connection_west=1",
    OakFence9 => "oak_fence" "minecraft:connection_east=1,minecraft:connection_north=0,minecraft:connection_south=0,minecraft:connection_west=1",
    OakFence10 => "oak_fence" "minecraft:connection_east=0,minecraft:connection_north=1,minecraft:connection_south=0,minecraft:connection_west=1",
    OakFence11 => "oak_fence" "minecraft:connection_east=1,minecraft:connection_north=1,minecraft:connection_south=0,minecraft:connection_west=1",
    OakFence12 => "oak_fence" "minecraft:connection_east=0,minecraft:connection_north=0,minecraft:connection_south=1,minecraft:connection_west=1",
    OakFence13 => "oak_fence" "minecraft:connection_east=1,minecraft:connection_north=0,minecraft:connection_south=1,minecraft:connection_west=1",
    OakFence14 => "oak_fence" "minecraft:connection_east=0,minecraft:connection_north=1,minecraft:connection_south=1,minecraft:connection_west=1",
    OakFence15 => "oak_fence" "minecraft:connection_east=1,minecraft:connection_north=1,minecraft:connection_south=1,minecraft:connection_west=1",
    DarkOakFence0 => "dark_oak_fence" "minecraft:connection_east=0,minecraft:connection_north=0,minecraft:connection_south=0,minecraft:connection_west=0",
    DarkOakFence1 => "dark_oak_fence" "minecraft:connection_east=1,minecraft:connection_north=0,minecraft:connection_south=0,minecraft:connection_west=0",
    DarkOakFence2 => "dark_oak_fence" "minecraft:connection_east=0,minecraft:connection_north=1,minecraft:connection_south=0,minecraft:connection_west=0",
    DarkOakFence3 => "dark_oak_fence" "minecraft:connection_east=1,minecraft:connection_north=1,minecraft:connection_south=0,minecraft:connection_west=0",
    DarkOakFence4 => "dark_oak_fence" "minecraft:connection_east=0,minecraft:connection_north=0,minecraft:connection_south=1,minecraft:connection_west=0",
    DarkOakFence5 => "dark_oak_fence" "minecraft:connection_east=1,minecraft:connection_north=0,minecraft:connection_south=1,minecraft:connection_west=0",
    DarkOakFence6 => "dark_oak_fence" "minecraft:connection_east=0,minecraft:connection_north=1,minecraft:connection_south=1,minecraft:connection_west=0",
    DarkOakFence7 => "dark_oak_fence" "minecraft:connection_east=1,minecraft:connection_north=1,minecraft:connection_south=1,minecraft:connection_west=0",
    DarkOakFence8 => "dark_oak_fence" "minecraft:connection_east=0,minecraft:connection_north=0,minecraft:connection_south=0,minecraft:connection_west=1",
    DarkOakFence9 => "dark_oak_fence" "minecraft:connection_east=1,minecraft:connection_north=0,minecraft:connection_south=0,minecraft:connection_west=1",
    DarkOakFence10 => "dark_oak_fence" "minecraft:connection_east=0,minecraft:connection_north=1,minecraft:connection_south=0,minecraft:connection_west=1",
    DarkOakFence11 => "dark_oak_fence" "minecraft:connection_east=1,minecraft:connection_north=1,minecraft:connection_south=0,minecraft:connection_west=1",
    DarkOakFence12 => "dark_oak_fence" "minecraft:connection_east=0,minecraft:connection_north=0,minecraft:connection_south=1,minecraft:connection_west=1",
    DarkOakFence13 => "dark_oak_fence" "minecraft:connection_east=1,minecraft:connection_north=0,minecraft:connection_south=1,minecraft:connection_west=1",
    DarkOakFence14 => "dark_oak_fence" "minecraft:connection_east=0,minecraft:connection_north=1,minecraft:connection_south=1,minecraft:connection_west=1",
    DarkOakFence15 => "dark_oak_fence" "minecraft:connection_east=1,minecraft:connection_north=1,minecraft:connection_south=1,minecraft:connection_west=1",
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
    Deadbush => "deadbush" "",
    Reeds => "reeds" "age=0",
    Cactus => "cactus" "age=0",
    BrownMushroom => "brown_mushroom" "",
    RedMushroom => "red_mushroom" "",
    Waterlily => "waterlily" "",
    MelonBlock => "melon_block" "",
    PumpkinSouth => "pumpkin" "minecraft:cardinal_direction=south",
    PumpkinWest => "pumpkin" "minecraft:cardinal_direction=west",
    PumpkinNorth => "pumpkin" "minecraft:cardinal_direction=north",
    PumpkinEast => "pumpkin" "minecraft:cardinal_direction=east",
    SweetBerryBush0 => "sweet_berry_bush" "growth=0",
    SweetBerryBush1 => "sweet_berry_bush" "growth=1",
    SweetBerryBush2 => "sweet_berry_bush" "growth=2",
    SweetBerryBush3 => "sweet_berry_bush" "growth=3",
    SweetBerryBush4 => "sweet_berry_bush" "growth=4",
    SweetBerryBush5 => "sweet_berry_bush" "growth=5",
    SweetBerryBush6 => "sweet_berry_bush" "growth=6",
    SweetBerryBush7 => "sweet_berry_bush" "growth=7",
    Poppy => "poppy" "",
    Dandelion => "dandelion" "",
    BlueOrchid => "blue_orchid" "",
    AzureBluet => "azure_bluet" "",
    RedTulip => "red_tulip" "",
    OrangeTulip => "orange_tulip" "",
    WhiteTulip => "white_tulip" "",
    PinkTulip => "pink_tulip" "",
    OxeyeDaisy => "oxeye_daisy" "",
    Cornflower => "cornflower" "",
    LilyOfTheValley => "lily_of_the_valley" "",
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
        self >= Block::Seagrass || matches!(self, Block::Template(i) if TEMPLATE_BLOCKS[i as usize].2)
    }

    pub fn is_kelp(self) -> bool {
        self >= Block::Kelp0
    }

    pub fn is_solid(self) -> bool {
        match self {
            Block::Template(i) => TEMPLATE_BLOCKS[i as usize].3 & COLLIDES != 0,
            _ => !matches!(self, Block::Air | Block::Water | Block::Lava) && !self.is_plant(),
        }
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
