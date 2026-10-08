//! The solid blocks of the cave biomes' surface builders and features, after table.rs's and before
//! plant_table.rs's. Speleothems run thickness tip, frustum, middle, base, merge, each standing
//! then hanging (`Block::speleothem`).

macro_rules! with_cave_blocks {
    ($($plants:tt)*) => {
        with_solid_blocks! {
            Sulfur => "sulfur" "",
            Cinnabar => "cinnabar" "",
            DripstoneBlock => "dripstone_block" "",
            PointedDripstone => "pointed_dripstone" "dripstone_thickness=tip,hanging=0",
            PointedDripstoneTipHanging => "pointed_dripstone" "dripstone_thickness=tip,hanging=1",
            PointedDripstoneFrustum => "pointed_dripstone" "dripstone_thickness=frustum,hanging=0",
            PointedDripstoneFrustumHanging => "pointed_dripstone" "dripstone_thickness=frustum,hanging=1",
            PointedDripstoneMiddle => "pointed_dripstone" "dripstone_thickness=middle,hanging=0",
            PointedDripstoneMiddleHanging => "pointed_dripstone" "dripstone_thickness=middle,hanging=1",
            PointedDripstoneBase => "pointed_dripstone" "dripstone_thickness=base,hanging=0",
            PointedDripstoneBaseHanging => "pointed_dripstone" "dripstone_thickness=base,hanging=1",
            PointedDripstoneMerge => "pointed_dripstone" "dripstone_thickness=merge,hanging=0",
            PointedDripstoneMergeHanging => "pointed_dripstone" "dripstone_thickness=merge,hanging=1",
            SulfurSpike => "sulfur_spike" "dripstone_thickness=tip,hanging=0",
            SulfurSpikeTipHanging => "sulfur_spike" "dripstone_thickness=tip,hanging=1",
            SulfurSpikeFrustum => "sulfur_spike" "dripstone_thickness=frustum,hanging=0",
            SulfurSpikeFrustumHanging => "sulfur_spike" "dripstone_thickness=frustum,hanging=1",
            SulfurSpikeMiddle => "sulfur_spike" "dripstone_thickness=middle,hanging=0",
            SulfurSpikeMiddleHanging => "sulfur_spike" "dripstone_thickness=middle,hanging=1",
            SulfurSpikeBase => "sulfur_spike" "dripstone_thickness=base,hanging=0",
            SulfurSpikeBaseHanging => "sulfur_spike" "dripstone_thickness=base,hanging=1",
            SulfurSpikeMerge => "sulfur_spike" "dripstone_thickness=merge,hanging=0",
            SulfurSpikeMergeHanging => "sulfur_spike" "dripstone_thickness=merge,hanging=1",
            $($plants)*
        }
    };
}
