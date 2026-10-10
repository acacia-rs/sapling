//! The bite, slain, chase and sick drills of bds_command_probe: a zombie hunting the actor
//! (docs/research/mob-attacks.md), and what food does besides feeding (food-effects.md).

pub const BITE_SHOWN: &[&str] = &[
    "UpdateAttributes", "EntityEvent", "SetEntityData", "SetHealth", "Respawn", "LevelSoundEvent", "MovePlayer", "SetEntityMotion", "DeathInfo", "Animate", "RemoveEntity", "AddEntity",
    "MoveEntityDelta", "MoveEntityAbsolute", "MotionPredictionHints", "CorrectPlayerMovePrediction", "MobEquipment",
];


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

/// The actor and a zombie 8 blocks off with something between them: a step one block high
/// across the floor, then one two high, then a wall five wide and two high.
pub fn chase() -> Vec<String> {
    let mut lines: Vec<String> = FLOOR.map(String::from).into();
    for between in ["fill ~-3 ~ ~-4 ~12 ~ ~-4 stone", "fill ~-3 ~ ~-4 ~12 ~1 ~-4 stone", "fill ~-2 ~ ~-4 ~2 ~1 ~-4 stone"] {
        lines.extend([between, "summon zombie ~ ~ ~-8"].map(String::from));
        lines.extend(std::iter::repeat_n(String::new(), 7));
        lines.extend(["kill @e[type=zombie]", "fill ~-9 ~ ~-12 ~9 ~1 ~-1 air", "effect @s instant_health 1 4", "", ""].map(String::from));
    }
    lines.extend(["gamemode creative @s", "kill @e[type=!player]", ""].map(String::from));
    lines
}

/// The actor, in survival and kept hungry: eats a spider eye, five of rotten flesh, three raw
/// chickens, three poisonous potatoes and a golden apple, then, poisoned, drinks milk.
pub fn sick() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "", "fill ~-3 ~-1 ~-3 ~8 ~-1 ~8 stone", "fill ~-3 ~ ~-3 ~8 ~6 ~8 air", "gamemode survival @s", ""].map(String::from).into();
    let eat = |lines: &mut Vec<String>, food: &str, after: usize| {
        lines.extend(["effect @s hunger 1 255 true".into(), String::new(), format!("replaceitem entity @s slot.weapon.mainhand 0 {food} 1"), "!eat".into()]);
        lines.extend(std::iter::repeat_n(String::new(), after));
    };
    eat(&mut lines, "spider_eye", 8);
    lines.extend(["effect @s clear", "effect @s instant_health 1 4"].map(String::from));
    for (food, times) in [("rotten_flesh", 5), ("chicken", 3), ("poisonous_potato", 3)] {
        (0..times).for_each(|_| eat(&mut lines, food, 3));
        lines.extend(["", "", "effect @s clear", "effect @s instant_health 1 4"].map(String::from));
    }
    eat(&mut lines, "golden_apple", 8);
    lines.extend(["effect @s clear", "effect @s poison 30 0", "effect @s speed 30 0", ""].map(String::from));
    eat(&mut lines, "milk_bucket", 4);
    lines.extend(["gamemode creative @s", "clear @s", ""].map(String::from));
    lines
}
