//! The Nether's blocks, after every other table's: a block hash taken over the overworld (tools/bench.sh)
//! hashes discriminants, so new blocks go last. Solids first, then the plants from `CrimsonRoots` on
//! (`Block::is_plant`).

macro_rules! with_nether_blocks {
    ($($plants:tt)*) => {
        with_multiface_blocks! {
            $($plants)*
            Netherrack => "netherrack" "",
            SoulSand => "soul_sand" "",
            SoulSoil => "soul_soil" "",
            Basalt => "basalt" "pillar_axis=y",
            Blackstone => "blackstone" "",
            CrimsonNylium => "crimson_nylium" "",
            WarpedNylium => "warped_nylium" "",
            NetherWartBlock => "nether_wart_block" "",
            QuartzOre => "quartz_ore" "",
            NetherGoldOre => "nether_gold_ore" "",
            AncientDebris => "ancient_debris" "",
            Glowstone => "glowstone" "",
            CrimsonStem => "crimson_stem" "pillar_axis=y",
            WarpedStem => "warped_stem" "pillar_axis=y",
            WarpedWartBlock => "warped_wart_block" "",
            Shroomlight => "shroomlight" "",
            // By `nextInt(4)` as `cardinal_direction`.
            DriedGhastSouth => "dried_ghast" "minecraft:cardinal_direction=south,rehydration_level=0",
            DriedGhastWest => "dried_ghast" "minecraft:cardinal_direction=west,rehydration_level=0",
            DriedGhastNorth => "dried_ghast" "minecraft:cardinal_direction=north,rehydration_level=0",
            DriedGhastEast => "dried_ghast" "minecraft:cardinal_direction=east,rehydration_level=0",
            CrimsonRoots => "crimson_roots" "",
            WarpedRoots => "warped_roots" "",
            NetherSprouts => "nether_sprouts" "",
            CrimsonFungus => "crimson_fungus" "",
            WarpedFungus => "warped_fungus" "",
            Fire => "fire" "age=0",
            SoulFire => "soul_fire" "age=0",
            // By age 0..=25 (`Block::weeping_vines`, `Block::twisting_vines`).
            WeepingVines0 => "weeping_vines" "weeping_vines_age=0", WeepingVines1 => "weeping_vines" "weeping_vines_age=1",
            WeepingVines2 => "weeping_vines" "weeping_vines_age=2", WeepingVines3 => "weeping_vines" "weeping_vines_age=3",
            WeepingVines4 => "weeping_vines" "weeping_vines_age=4", WeepingVines5 => "weeping_vines" "weeping_vines_age=5",
            WeepingVines6 => "weeping_vines" "weeping_vines_age=6", WeepingVines7 => "weeping_vines" "weeping_vines_age=7",
            WeepingVines8 => "weeping_vines" "weeping_vines_age=8", WeepingVines9 => "weeping_vines" "weeping_vines_age=9",
            WeepingVines10 => "weeping_vines" "weeping_vines_age=10", WeepingVines11 => "weeping_vines" "weeping_vines_age=11",
            WeepingVines12 => "weeping_vines" "weeping_vines_age=12", WeepingVines13 => "weeping_vines" "weeping_vines_age=13",
            WeepingVines14 => "weeping_vines" "weeping_vines_age=14", WeepingVines15 => "weeping_vines" "weeping_vines_age=15",
            WeepingVines16 => "weeping_vines" "weeping_vines_age=16", WeepingVines17 => "weeping_vines" "weeping_vines_age=17",
            WeepingVines18 => "weeping_vines" "weeping_vines_age=18", WeepingVines19 => "weeping_vines" "weeping_vines_age=19",
            WeepingVines20 => "weeping_vines" "weeping_vines_age=20", WeepingVines21 => "weeping_vines" "weeping_vines_age=21",
            WeepingVines22 => "weeping_vines" "weeping_vines_age=22", WeepingVines23 => "weeping_vines" "weeping_vines_age=23",
            WeepingVines24 => "weeping_vines" "weeping_vines_age=24", WeepingVines25 => "weeping_vines" "weeping_vines_age=25",
            TwistingVines0 => "twisting_vines" "twisting_vines_age=0", TwistingVines1 => "twisting_vines" "twisting_vines_age=1",
            TwistingVines2 => "twisting_vines" "twisting_vines_age=2", TwistingVines3 => "twisting_vines" "twisting_vines_age=3",
            TwistingVines4 => "twisting_vines" "twisting_vines_age=4", TwistingVines5 => "twisting_vines" "twisting_vines_age=5",
            TwistingVines6 => "twisting_vines" "twisting_vines_age=6", TwistingVines7 => "twisting_vines" "twisting_vines_age=7",
            TwistingVines8 => "twisting_vines" "twisting_vines_age=8", TwistingVines9 => "twisting_vines" "twisting_vines_age=9",
            TwistingVines10 => "twisting_vines" "twisting_vines_age=10", TwistingVines11 => "twisting_vines" "twisting_vines_age=11",
            TwistingVines12 => "twisting_vines" "twisting_vines_age=12", TwistingVines13 => "twisting_vines" "twisting_vines_age=13",
            TwistingVines14 => "twisting_vines" "twisting_vines_age=14", TwistingVines15 => "twisting_vines" "twisting_vines_age=15",
            TwistingVines16 => "twisting_vines" "twisting_vines_age=16", TwistingVines17 => "twisting_vines" "twisting_vines_age=17",
            TwistingVines18 => "twisting_vines" "twisting_vines_age=18", TwistingVines19 => "twisting_vines" "twisting_vines_age=19",
            TwistingVines20 => "twisting_vines" "twisting_vines_age=20", TwistingVines21 => "twisting_vines" "twisting_vines_age=21",
            TwistingVines22 => "twisting_vines" "twisting_vines_age=22", TwistingVines23 => "twisting_vines" "twisting_vines_age=23",
            TwistingVines24 => "twisting_vines" "twisting_vines_age=24", TwistingVines25 => "twisting_vines" "twisting_vines_age=25",
        }
    };
}

impl super::Block {
    pub fn weeping_vines(age: i32) -> Self {
        Self::ALL[Self::WeepingVines0.index() + age as usize]
    }

    pub fn twisting_vines(age: i32) -> Self {
        Self::ALL[Self::TwistingVines0.index() + age as usize]
    }

    pub fn dried_ghast(direction: i32) -> Self {
        Self::ALL[Self::DriedGhastSouth.index() + direction as usize]
    }
}
