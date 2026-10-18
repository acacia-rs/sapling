//! The End's blocks, after the Nether's (nether_table.rs says why new blocks go last).

macro_rules! with_end_blocks {
    ($($plants:tt)*) => {
        with_nether_blocks! {
            {$($plants)*}
            {
                EndStone => "end_stone" "",
            }
        }
    };
}
