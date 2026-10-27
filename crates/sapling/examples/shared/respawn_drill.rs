//! The anchor drill of bds_command_probe: a respawn anchor charged, made a spawn and risen by in
//! the Nether, the rule `doImmediateRespawn`, and the last death position BDS tells.

pub const ANCHOR_SHOWN: &[&str] = &[
    "UpdateAttributes", "EntityEvent", "SetEntityData", "SetHealth", "Respawn", "LevelSoundEvent", "LevelEvent", "MovePlayer", "Text", "DeathInfo", "PlayerAction", "SetSpawnPosition",
    "CorrectPlayerMovePrediction", "PlayStatus", "ChangeDimension", "UpdateBlock", "UpdateSubchunkBlocks", "GameRulesChanged", "InventorySlot", "MobEquipment",
];

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

fn wait(lines: &mut Vec<String>, seconds: usize) {
    lines.extend(std::iter::repeat_n(String::new(), seconds));
}

/// The actor in survival in the Nether: charges an anchor twice with glowstone, clicks it with an
/// empty hand twice (its spawn), dies three times (two charges spent, then none left), then in the
/// overworld dies with `doImmediateRespawn` on.
pub fn anchor() -> Vec<String> {
    // Peaceful and no spawning: the Nether's ghasts shot the actor in the first run.
    let mut lines = owned(["difficulty peaceful", "gamerule domobspawning false", "execute in nether run tp @s 0.5 100 0.5", "", "", "kill @e[type=!player]", "fill -3 99 -3 6 99 6 stone"]);
    lines.extend(owned(["fill -3 100 -3 6 104 6 air", "tp @s 0.5 100 0.5", "", "gamemode survival @s", ""]));
    lines.extend(owned(["setblock ~2 ~ ~ respawn_anchor", "replaceitem entity @s slot.weapon.mainhand 0 glowstone 2", "", "!click 2 0 0", "", "!click 2 0 0", ""]));
    lines.extend(owned(["replaceitem entity @s slot.weapon.mainhand 0 air", "", "!click 2 0 0", "", "!click 2 0 0", "", "tp @s ~-2 ~ ~2", "", "kill @s"]));
    wait(&mut lines, 6);
    lines.push("kill @s".into());
    wait(&mut lines, 6);
    lines.push("kill @s".into());
    wait(&mut lines, 8);
    lines.extend(owned(["gamerule doimmediaterespawn true", "", "kill @s"]));
    wait(&mut lines, 5);
    lines.extend(owned(["gamerule doimmediaterespawn false", "gamemode creative @s", "difficulty easy", "gamerule domobspawning true", ""]));
    lines
}
