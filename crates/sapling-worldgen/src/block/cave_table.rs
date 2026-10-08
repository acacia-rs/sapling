//! The solid blocks of the cave biomes' surface builders and features, after table.rs's and before
//! plant_table.rs's.

macro_rules! with_cave_blocks {
    ($($plants:tt)*) => {
        with_solid_blocks! {
            Sulfur => "sulfur" "",
            Cinnabar => "cinnabar" "",
            $($plants)*
        }
    };
}
