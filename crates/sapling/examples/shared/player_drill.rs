//! The hurt, smother, hunger, eat, tire, bite and slain drills of bds_command_probe: a player harmed,
//! killed and risen again (docs/research/player-damage.md, mob-attacks.md).

use std::time::Duration;

use acacia_bot::Bot;

/// Does what a drill's `!` line asks of the actor: `!eat` holds use on what it holds, `!jump`,
/// `!walk` and `!sprint` hold those keys until `!stop`. False for any other line.
pub async fn act(actor: &mut Bot, line: &str) -> bool {
    let keys = |actor: &mut Bot, forward: f32, sprint: bool, jump: bool| {
        if let Some(controls) = actor.controls() {
            (controls.forward, controls.sprint, controls.jump) = (forward, sprint, jump);
        }
    };
    match line {
        // The use goes on as the bot is polled after (`Bot::consume`).
        "!eat" => drop(tokio::time::timeout(Duration::from_millis(1), actor.consume()).await),
        "!jump" => keys(actor, 0.0, false, true),
        "!walk" => keys(actor, 1.0, false, false),
        "!sprint" => keys(actor, 1.0, true, false),
        "!leap" => keys(actor, 1.0, true, true),
        "!stop" => keys(actor, 0.0, false, false),
        _ => return false,
    }
    true
}

pub const HURT_SHOWN: &[&str] = &[
    "UpdateAttributes", "EntityEvent", "SetEntityData", "SetHealth", "Respawn", "LevelSoundEvent", "LevelEvent", "MovePlayer", "SetEntityMotion", "Text", "DeathInfo", "PlayerAction", "Animate",
    "MobEffect", "SetPlayerGameType", "UpdatePlayerGameType", "RemoveEntity", "AddPlayer", "AddItemEntity", "AddEntity", "InventoryContent", "InventorySlot", "MobEquipment", "MobArmorEquipment",
    "SetSpawnPosition", "CorrectPlayerMovePrediction", "PlayStatus", "ChangeDimension", "SpawnExperienceOrb", "CompletedUsingItem", "ItemStackResponse",
];

pub const BITE_SHOWN: &[&str] = &[
    "UpdateAttributes", "EntityEvent", "SetEntityData", "SetHealth", "Respawn", "LevelSoundEvent", "MovePlayer", "SetEntityMotion", "DeathInfo", "Animate", "RemoveEntity", "AddEntity",
    "MoveEntityDelta", "MoveEntityAbsolute", "MotionPredictionHints", "CorrectPlayerMovePrediction", "MobEquipment",
];

/// The lines and shown packets of the drill `name`, if it is one of these.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "hurt" => (hurt(), HURT_SHOWN),
        "smother" => (smother(), HURT_SHOWN),
        "hunger" => (hunger(), HURT_SHOWN),
        "eat" => (eat(), HURT_SHOWN),
        "tire" => (tire(), HURT_SHOWN),
        "bite" => (bite(), BITE_SHOWN),
        "slain" => (slain(), BITE_SHOWN),
        _ => return None,
    })
}

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

/// The actor, in survival at the end of a 60 block floor: jumps in place, walks, sprints and
/// sprints jumping, five seconds each, back at the start between.
pub fn tire() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "", "fill ~-3 ~-1 ~-3 ~3 ~-1 ~60 stone", "fill ~-3 ~ ~-3 ~3 ~6 ~60 air", "gamemode survival @s", ""].map(String::from).into();
    let wait = |lines: &mut Vec<String>, seconds: usize| lines.extend(std::iter::repeat_n(String::new(), seconds));
    for key in ["!jump", "!walk", "!sprint", "!leap"] {
        lines.push(key.into());
        wait(&mut lines, 5);
        lines.extend(["!stop", ""].map(String::from));
        lines.push(if key == "!jump" { String::new() } else { "tp @s ~ ~ ~-30".into() });
        wait(&mut lines, 2);
    }
    lines.extend(["gamemode creative @s", ""].map(String::from));
    lines
}

const FLOOR: [&str; 8] = ["kill @e[type=!player]", "gamerule domobspawning false", "difficulty easy", "fill ~-3 ~-1 ~-25 ~12 ~-1 ~70 stone", "fill ~-3 ~ ~-25 ~12 ~6 ~70 air", "gamemode survival @s", "tp @s ~6 ~ ~20", ""];

/// The actor, with 4 health left, and a zombie 3 blocks off that kills it.
pub fn slain() -> Vec<String> {
    let mut lines: Vec<String> = FLOOR.map(String::from).into();
    lines.extend(["damage @s 16", "summon zombie ~ ~ ~-3"].map(String::from));
    lines.extend(std::iter::repeat_n(String::new(), 8));
    lines.extend(["gamemode creative @s", "kill @e[type=!player]", ""].map(String::from));
    lines
}

/// The actor, in survival on a wide floor and away from the watcher: a zombie 8 blocks off comes
/// and hits it, each hit throwing it further; zombies 30 and 40 off; one beside it while it is in
/// creative, then in survival.
pub fn bite() -> Vec<String> {
    let mut lines: Vec<String> = FLOOR.map(String::from).into();
    let wait = |lines: &mut Vec<String>, seconds: usize| lines.extend(std::iter::repeat_n(String::new(), seconds));
    lines.push("summon zombie ~ ~ ~-8".into());
    wait(&mut lines, 5);
    for far in [30, 40] {
        lines.extend(["kill @e[type=zombie]".into(), "effect @s instant_health 1 4".into(), String::new(), String::new(), format!("summon zombie ~ ~ ~-{far}")]);
        wait(&mut lines, 6);
    }
    lines.extend(["kill @e[type=zombie]", "effect @s instant_health 1 4", "gamemode creative @s", "", "", "summon zombie ~ ~ ~-4"].map(String::from));
    wait(&mut lines, 5);
    lines.push("gamemode survival @s".into());
    wait(&mut lines, 4);
    lines.extend(["gamemode creative @s", "kill @e[type=!player]", ""].map(String::from));
    lines
}
