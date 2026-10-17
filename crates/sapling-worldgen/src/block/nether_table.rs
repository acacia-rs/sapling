//! The Nether's terrain and surface blocks, after every other table's: a block hash taken over the
//! overworld (tools/bench.sh) hashes discriminants, so new blocks go last.

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
        }
    };
}
