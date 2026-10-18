//! The End's blocks, after the Nether's (nether_table.rs says why new blocks go last).

macro_rules! with_end_blocks {
    ($($plants:tt)*) => {
        with_nether_blocks! {
            {$($plants)*}
            {
                EndStone => "end_stone" "",
                ChorusPlant => "chorus_plant" "",
                ChorusFlower5 => "chorus_flower" "age=5",
                EndGateway => "end_gateway" "",
                InfiniburnBedrock => "bedrock" "infiniburn_bit=1",
            }
        }
    };
}
