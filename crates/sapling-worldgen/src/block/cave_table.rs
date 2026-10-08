//! The solid blocks of the cave biomes' surface builders and features (and coral), after table.rs's and before
//! plant_table.rs's. Speleothems run thickness tip, frustum, middle, base, merge, each standing
//! then hanging (`Block::speleothem`).

macro_rules! with_cave_blocks {
    ($($plants:tt)*) => {
        with_solid_blocks! {
            Sulfur => "sulfur" "",
            Cinnabar => "cinnabar" "",
            PotentSulfur => "potent_sulfur" "potent_sulfur_state=wet",
            DripstoneBlock => "dripstone_block" "",
            Magma => "magma" "",
            MobSpawner => "mob_spawner" "",
            Sculk => "sculk" "",
            SculkCatalyst => "sculk_catalyst" "bloom=0",
            SculkSensor => "sculk_sensor" "sculk_sensor_phase=0",
            SculkShrieker => "sculk_shrieker" "active=0,can_summon=1",
            // Warm ocean coral by `CoralColor` (tube, brain, bubble, fire, horn), live then dead (feature/coral).
            TubeCoralBlock => "tube_coral_block" "", BrainCoralBlock => "brain_coral_block" "",
            BubbleCoralBlock => "bubble_coral_block" "", FireCoralBlock => "fire_coral_block" "",
            HornCoralBlock => "horn_coral_block" "",
            DeadTubeCoralBlock => "dead_tube_coral_block" "", DeadBrainCoralBlock => "dead_brain_coral_block" "",
            DeadBubbleCoralBlock => "dead_bubble_coral_block" "", DeadFireCoralBlock => "dead_fire_coral_block" "",
            DeadHornCoralBlock => "dead_horn_coral_block" "",
            // By Bedrock's Direction: south, west, north, east (`Block::CHESTS`).
            ChestSouth => "chest" "minecraft:cardinal_direction=south",
            ChestWest => "chest" "minecraft:cardinal_direction=west",
            ChestNorth => "chest" "minecraft:cardinal_direction=north",
            ChestEast => "chest" "minecraft:cardinal_direction=east",
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
