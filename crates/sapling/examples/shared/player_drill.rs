//! The hurt drill of bds_command_probe: a player harmed, killed and risen again
//! (docs/research/player-damage.md).

pub const HURT_SHOWN: &[&str] = &[
    "UpdateAttributes", "EntityEvent", "SetEntityData", "SetHealth", "Respawn", "LevelSoundEvent", "LevelEvent", "MovePlayer", "SetEntityMotion", "Text", "DeathInfo", "PlayerAction", "Animate",
    "MobEffect", "SetPlayerGameType", "UpdatePlayerGameType", "RemoveEntity", "AddPlayer", "AddItemEntity", "AddEntity", "InventoryContent", "InventorySlot", "MobEquipment", "MobArmorEquipment",
    "SetSpawnPosition", "CorrectPlayerMovePrediction", "PlayStatus", "ChangeDimension", "SpawnExperienceOrb",
];

/// The actor, in survival on a stone floor: harmed by `/damage` with and without a cause, twice at
/// once, dropped 6 and 12 blocks, stood in fire and in lava and put out by water, left to mend,
/// killed holding a stack and risen, then killed by a fall.
pub fn hurt() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "", "fill ~-3 ~-4 ~-3 ~8 ~-1 ~8 stone", "fill ~-3 ~ ~-3 ~8 ~14 ~8 air", "gamemode survival @s", ""].map(String::from).into();
    let wait = |lines: &mut Vec<String>, seconds: usize| lines.extend(std::iter::repeat_n(String::new(), seconds));
    lines.extend(["damage @s 3", "", "damage @s 2 fall", "", "damage @s 1 fire", "", "damage @s 1 drowning", "", "damage @s 1 entity_attack", ""].map(String::from));
    lines.extend(["damage @s 1", "damage @s 1"].map(String::from));
    wait(&mut lines, 2);
    lines.extend(["effect @s instant_health 1 4", "", "tp @s ~ ~6 ~"].map(String::from));
    wait(&mut lines, 3);
    lines.extend(["effect @s instant_health 1 4", "", "tp @s ~ ~12 ~"].map(String::from));
    wait(&mut lines, 4);
    lines.extend(["effect @s instant_health 1 4", "", "setblock ~ ~-1 ~ netherrack", "setblock ~ ~ ~ fire"].map(String::from));
    wait(&mut lines, 3);
    lines.extend(["setblock ~ ~ ~ air", "setblock ~ ~-1 ~ stone"].map(String::from));
    wait(&mut lines, 10);
    lines.extend(["effect @s instant_health 1 4", "", "setblock ~ ~ ~ lava"].map(String::from));
    wait(&mut lines, 2);
    lines.push("setblock ~ ~ ~ water".into());
    wait(&mut lines, 2);
    lines.push("setblock ~ ~ ~ air".into());
    wait(&mut lines, 12);
    lines.extend(["give @s stone 5", "xp 30 @s", "", "kill @s"].map(String::from));
    wait(&mut lines, 8);
    lines.extend(["gamemode survival @s", "tp @s ~ ~40 ~"].map(String::from));
    wait(&mut lines, 10);
    lines.extend(["gamemode creative @s", "kill @e[type=!player]", ""].map(String::from));
    lines
}
