//! The hurt, smother, hunger and eat drills of bds_command_probe: a player harmed, killed and risen again
//! (docs/research/player-damage.md).

pub const HURT_SHOWN: &[&str] = &[
    "UpdateAttributes", "EntityEvent", "SetEntityData", "SetHealth", "Respawn", "LevelSoundEvent", "LevelEvent", "MovePlayer", "SetEntityMotion", "Text", "DeathInfo", "PlayerAction", "Animate",
    "MobEffect", "SetPlayerGameType", "UpdatePlayerGameType", "RemoveEntity", "AddPlayer", "AddItemEntity", "AddEntity", "InventoryContent", "InventorySlot", "MobEquipment", "MobArmorEquipment",
    "SetSpawnPosition", "CorrectPlayerMovePrediction", "PlayStatus", "ChangeDimension", "SpawnExperienceOrb", "CompletedUsingItem", "ItemStackResponse",
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

/// The actor, in survival in a glass cell: its head under water until it drowns a while, given
/// air, then its head in stone, then both blocks of it.
pub fn smother() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "", "fill ~-3 ~-4 ~-3 ~8 ~-1 ~8 stone", "fill ~-3 ~ ~-3 ~8 ~14 ~8 air", "gamemode survival @s", ""].map(String::from).into();
    let wait = |lines: &mut Vec<String>, seconds: usize| lines.extend(std::iter::repeat_n(String::new(), seconds));
    lines.extend(["fill ~-1 ~ ~-1 ~1 ~2 ~1 glass", "fill ~ ~ ~ ~ ~1 ~ water"].map(String::from));
    wait(&mut lines, 22);
    lines.push("fill ~ ~ ~ ~ ~1 ~ air".into());
    wait(&mut lines, 4);
    lines.extend(["effect @s instant_health 1 4", "", "setblock ~ ~1 ~ stone"].map(String::from));
    wait(&mut lines, 4);
    lines.push("setblock ~ ~1 ~ air".into());
    wait(&mut lines, 2);
    lines.extend(["effect @s instant_health 1 4", "", "fill ~ ~ ~ ~ ~1 ~ stone"].map(String::from));
    wait(&mut lines, 3);
    lines.extend(["gamemode creative @s", "fill ~-1 ~ ~-1 ~1 ~2 ~1 air", ""].map(String::from));
    lines
}

/// The actor, in survival: hurt 14 and left to mend until its hunger stops that, its hunger
/// drained by the effect and left to starve, fed by the saturation effect, and hurt again.
pub fn hunger() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "", "fill ~-3 ~-4 ~-3 ~8 ~-1 ~8 stone", "fill ~-3 ~ ~-3 ~8 ~14 ~8 air", "gamemode survival @s", ""].map(String::from).into();
    let wait = |lines: &mut Vec<String>, seconds: usize| lines.extend(std::iter::repeat_n(String::new(), seconds));
    lines.push("damage @s 14".into());
    wait(&mut lines, 45);
    lines.push("effect @s hunger 12 255 true".into());
    wait(&mut lines, 40);
    lines.push("effect @s saturation 1 9 true".into());
    wait(&mut lines, 12);
    lines.extend(["damage @s 4", "", "damage @s 1 fall"].map(String::from));
    wait(&mut lines, 10);
    lines.extend(["gamemode creative @s", ""].map(String::from));
    lines
}

/// The actor, in survival, hurt and made hungry: eats two apples, a steak put in its hand, and tries
/// one more with its hunger full. `!eat` holds use on what it holds.
pub fn eat() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "", "fill ~-3 ~-4 ~-3 ~8 ~-1 ~8 stone", "fill ~-3 ~ ~-3 ~8 ~14 ~8 air", "gamemode survival @s", ""].map(String::from).into();
    let wait = |lines: &mut Vec<String>, seconds: usize| lines.extend(std::iter::repeat_n(String::new(), seconds));
    // Mending takes a hunger point in 17 seconds, where there is no hunger effect to.
    lines.push("damage @s 14".into());
    wait(&mut lines, 18);
    lines.extend(["give @s apple 3", "effect @s hunger 2 255 true"].map(String::from));
    wait(&mut lines, 4);
    for _ in 0..2 {
        lines.push("!eat".into());
        wait(&mut lines, 4);
    }
    lines.extend(["replaceitem entity @s slot.weapon.mainhand 0 cooked_beef 2", "", "!eat"].map(String::from));
    wait(&mut lines, 4);
    lines.extend(["effect @s saturation 1 19 true", "", "!eat"].map(String::from));
    wait(&mut lines, 4);
    lines.extend(["gamemode creative @s", ""].map(String::from));
    lines
}
