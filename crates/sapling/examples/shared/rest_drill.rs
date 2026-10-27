//! The rest and spectate drills of bds_command_probe: monsters that keep a sleeper awake, sleep
//! in a storm by day, where a sleeper stands up when its spot is taken, and what a spectator
//! picks up.

pub const REST_SHOWN: &[&str] = &[
    "UpdateBlock", "UpdateSubchunkBlocks", "SetEntityData", "MovePlayer", "PlayerAction", "Animate", "Text", "SetSpawnPosition", "CorrectPlayerMovePrediction", "LevelEventGeneric", "SetTime",
    "LevelEvent", "SyncWorldClocks",
];
pub const SPECTATE_SHOWN: &[&str] = &["TakeItemEntity", "AddItemEntity", "AddEntity", "RemoveEntity", "InventorySlot", "InventoryContent", "SpawnExperienceOrb", "LevelSoundEvent", "UpdateAttributes"];

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

/// Back to the armour stand, where the actor first stood, and into the bed east of it.
const SLEEP: [&str; 3] = ["tp @s @e[type=armor_stand]", "", "!sleep 1 0 0"];

/// The actor in survival on a floor, an armour stand where it stands and a bed east of it (its
/// foot there, its head south of that). Night: a zombie caged 6 east of the bed (refused), then
/// 10 east (slept); a zombified piglin 3 off; a zombie caged 6 above. A storm by noon, then a
/// clear noon. A sleeper whose own cell is walled, and one walled all round.
pub fn rest() -> Vec<String> {
    let mut lines = owned(["difficulty easy", "gamerule domobspawning false", "kill @e[type=!player]", "fill ~-12 ~-1 ~-12 ~12 ~-1 ~12 stone", "fill ~-12 ~ ~-12 ~12 ~8 ~12 air"]);
    lines.extend(owned(["summon armor_stand ~ ~ ~", "setblock ~1 ~ ~ bed", "gamemode survival @s", "time set 18000", ""]));
    lines.extend(owned(["fill ~6 ~ ~-1 ~8 ~2 ~1 glass", "fill ~7 ~ ~ ~7 ~1 ~ air", "summon zombie ~7 ~ ~", ""]));
    lines.extend(SLEEP.map(String::from));
    lines.extend(owned(["", "", "kill @e[type=zombie]", "fill ~10 ~ ~-1 ~12 ~2 ~1 glass", "fill ~11 ~ ~ ~11 ~1 ~ air", "summon zombie ~11 ~ ~", ""]));
    lines.extend(SLEEP.map(String::from));
    lines.extend(owned(["", "", "!wake", "", "kill @e[type=zombie]", "summon zombie_pigman ~3 ~ ~3", ""]));
    lines.extend(SLEEP.map(String::from));
    lines.extend(owned(["", "", "!wake", "", "kill @e[type=zombie_pigman]", "fill ~-1 ~6 ~-1 ~1 ~8 ~1 glass", "fill ~ ~6 ~ ~ ~7 ~ air", "summon zombie ~ ~6 ~", ""]));
    lines.extend(SLEEP.map(String::from));
    lines.extend(owned(["", "", "!wake", "", "kill @e[type=zombie]", "fill ~-1 ~6 ~-1 ~1 ~8 ~1 air", "time set 6000", "weather thunder", ""]));
    lines.extend(SLEEP.map(String::from));
    lines.extend(owned(["", "", "", "", "", "", "", "weather clear", "time set 6000", ""]));
    lines.extend(SLEEP.map(String::from));
    lines.extend(owned(["", "", "time set 18000", ""]));
    lines.extend(SLEEP.map(String::from));
    lines.extend(owned(["", "execute at @e[type=armor_stand] run fill ~ ~ ~ ~ ~1 ~ stone", "", "!wake", "", "", "execute at @e[type=armor_stand] run fill ~ ~ ~ ~ ~1 ~ air"]));
    lines.extend(SLEEP.map(String::from));
    lines.extend(owned(["", "execute at @e[type=armor_stand] run fill ~-1 ~ ~-1 ~3 ~1 ~3 stone replace air", "", "!wake", "", ""]));
    lines.extend(owned(["execute at @e[type=armor_stand] run fill ~-1 ~ ~-1 ~3 ~1 ~3 air replace stone", "tp @s @e[type=armor_stand]", "kill @e[type=!player]", "weather clear", "time set 1000"]));
    lines.extend(owned(["gamemode creative @s", "gamerule domobspawning true", ""]));
    lines
}

/// The actor a spectator beside a dropped stack and three orbs, then in survival.
pub fn spectate() -> Vec<String> {
    // The watcher stands off, so the orbs have only the actor to go to; dirt drops by hand.
    let mut lines = owned(["gamerule domobspawning false", "kill @e[type=!player]", "fill ~-3 ~-1 ~-3 ~3 ~-1 ~3 dirt", "fill ~-3 ~ ~-3 ~3 ~3 ~3 air", "tp CmdWatcher ~30 ~ ~", "clear @s", "gamemode spectator @s", ""]);
    lines.extend(owned(["loot spawn ~1 ~ ~ mine ~ ~-1 ~", "summon xp_orb ~1 ~ ~", "summon xp_orb ~-1 ~ ~", "summon xp_orb ~ ~ ~1", "", "", "", "", "gamemode survival @s", "", "", "", "gamemode creative @s", "kill @e[type=!player]", "tp CmdWatcher @s", "gamerule domobspawning true", ""]));
    lines
}
