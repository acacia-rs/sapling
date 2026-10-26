//! The liquid drills of bds_command_probe (`concrete`, `water`, `lava`).

use std::time::Duration;

/// When concrete powder sets, on a stone stage: put beside water, dropped into a pool two deep and
/// one deep, water put beside it and on it.
pub(crate) const CONCRETE: &[&str] = &[
    "fill ~2 ~-4 ~6 ~14 ~-1 ~8 stone",
    "fill ~2 ~ ~6 ~14 ~4 ~8 air",
    // Water only in holes in the stone, where it stays.
    "setblock ~3 ~-1 ~7 water",
    "setblock ~4 ~-1 ~7 red_concrete_powder",
    "fill ~6 ~-2 ~7 ~6 ~-1 ~7 water",
    "setblock ~6 ~3 ~7 red_concrete_powder",
    "setblock ~13 ~-1 ~7 water",
    "setblock ~13 ~3 ~7 red_concrete_powder",
    "setblock ~8 ~-1 ~7 red_concrete_powder",
    "setblock ~9 ~-1 ~7 water",
    // Last: this water spreads over the stage.
    "setblock ~11 ~-1 ~7 red_concrete_powder",
    "setblock ~11 ~ ~7 water",
];
/// What water does to an item, an orb and an armour stand dropped into a pool three deep.
pub(crate) const WATER: &[&str] = &[
    "fill ~2 ~-5 ~6 ~8 ~-1 ~12 stone",
    "fill ~2 ~ ~6 ~8 ~4 ~12 air",
    "fill ~3 ~-4 ~7 ~7 ~-1 ~9 water",
    "setblock ~5 ~2 ~8 dirt",
    "setblock ~5 ~2 ~8 air destroy",
    "summon xp_orb ~6.5 ~1 ~8.5",
    "summon armor_stand ~3.5 ~1 ~8.5",
];
pub(crate) const WATER_SHOWN: &[&str] = &["AddItemEntity", "AddEntity", "RemoveEntity", "MoveEntityDelta", "SetEntityMotion", "MotionPredictionHints", "LevelSoundEvent", "SetEntityData"];
/// The water drill over a pool of lava, which also burns what falls in.
pub(crate) const LAVA: &[&str] = &[
    "fill ~2 ~-5 ~6 ~8 ~-1 ~12 stone",
    "fill ~2 ~ ~6 ~8 ~4 ~12 air",
    "fill ~3 ~-4 ~7 ~7 ~-1 ~9 lava",
    "setblock ~5 ~2 ~8 dirt",
    "setblock ~5 ~2 ~8 air destroy",
    // Mid-pool: /summon throws an orb up to about two blocks.
    "summon xp_orb ~5.5 ~1 ~8.5",
    "summon armor_stand ~3.5 ~1 ~8.5",
];
pub(crate) const LAVA_SHOWN: &[&str] = &[
    "AddItemEntity",
    "AddEntity",
    "RemoveEntity",
    "MoveEntityDelta",
    "SetEntityMotion",
    "MotionPredictionHints",
    "LevelSoundEvent",
    "EntityEvent",
    "SetEntityData",
    "LevelEvent",
];
/// Long enough to see a thing sink or rise through the pool.
pub(crate) const WATER_SETTLE: Duration = Duration::from_millis(8000);
pub(crate) const CONCRETE_SHOWN: &[&str] = &["UpdateBlock", "UpdateBlockSynced", "UpdateSubchunkBlocks", "AddEntity", "RemoveEntity", "AddItemEntity", "LevelEvent", "LevelSoundEvent"];
