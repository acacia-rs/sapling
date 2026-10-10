//! The bite, slain, chase, sick, worn, kinds and stroll drills of bds_command_probe: a zombie hunting the actor
//! (docs/research/mob-attacks.md), and what food does besides feeding (food-effects.md).

pub const BITE_SHOWN: &[&str] = &[
    "UpdateAttributes", "EntityEvent", "SetEntityData", "SetHealth", "Respawn", "LevelSoundEvent", "MovePlayer", "SetEntityMotion", "DeathInfo", "Animate", "RemoveEntity", "AddEntity",
    "MoveEntityDelta", "MoveEntityAbsolute", "MotionPredictionHints", "CorrectPlayerMovePrediction", "MobEquipment",
];


pub const KINDS_SHOWN: &[&str] = &[
    "UpdateAttributes", "EntityEvent", "SetEntityData", "LevelSoundEvent", "SetEntityMotion", "DeathInfo", "RemoveEntity", "AddEntity", "MoveEntityDelta", "MoveEntityAbsolute", "MobEffect", "MobEquipment",
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

/// The actor, in survival: hit bare by `/damage` as suffocation, then in all of iron for 2 by
/// each cause the command knows besides those `armour` tried, and last hit by a zombie four times.
pub fn worn() -> Vec<String> {
    let mut lines: Vec<String> = FLOOR.map(String::from).into();
    let heal = ["effect @s instant_health 1 4", ""].map(String::from);
    lines.extend(["damage @s 2 suffocation", ""].map(String::from));
    lines.extend(heal.clone());
    for (part, piece) in [("head", "helmet"), ("chest", "chestplate"), ("legs", "leggings"), ("feet", "boots")] {
        lines.push(format!("replaceitem entity @s slot.armor.{part} 0 iron_{piece}"));
    }
    let causes = [
        "suffocation", "fire_tick", "magic", "wither", "starve", "void", "contact", "projectile", "entity_explosion", "block_explosion", "lightning", "anvil", "falling_block", "thorns",
        "freezing", "stalactite", "stalagmite", "sonic_boom", "fly_into_wall", "magma", "campfire", "soul_campfire",
    ];
    for cause in causes {
        lines.extend([format!("damage @s 2 {cause}"), String::new()]);
        lines.extend(heal.clone());
    }
    lines.push("summon zombie ~ ~ ~-3".into());
    lines.extend(std::iter::repeat_n(String::new(), 6));
    lines.extend(["gamemode creative @s", "kill @e[type=!player]", "clear @s", ""].map(String::from));
    lines
}

/// The actor at midnight and, one after another, a husk, a spider, a vindicator, a wither
/// skeleton and a zombie villager 8 blocks off and a cave spider and a silverfish 4 off, each left
/// to hit it a few times.
pub fn kinds() -> Vec<String> {
    let mut lines: Vec<String> = FLOOR.map(String::from).into();
    lines.push("time set midnight".into());
    for (kind, far, seconds) in [("husk", 8, 6), ("spider", 8, 5), ("vindicator", 8, 3), ("wither_skeleton", 8, 4), ("zombie_villager", 8, 6), ("cave_spider", 4, 4), ("silverfish", 4, 4)] {
        lines.push(format!("summon {kind} ~ ~ ~-{far}"));
        lines.extend(std::iter::repeat_n(String::new(), seconds));
        lines.extend(["kill @e[type=!player]", "effect @s clear", "effect @s instant_health 1 4", "", ""].map(String::from));
    }
    lines.extend(["gamemode creative @s", "kill @e[type=!player]", ""].map(String::from));
    lines
}

/// The actor in creative on a floor 40 blocks square, a zombie and a cow left to themselves for a
/// minute, then the cow hit and watched a while longer.
pub fn stroll() -> Vec<String> {
    let mut lines: Vec<String> =
        ["kill @e[type=!player]", "gamerule domobspawning false", "difficulty easy", "time set midnight", "fill ~-20 ~-1 ~-20 ~20 ~-1 ~20 stone", "fill ~-20 ~ ~-20 ~20 ~4 ~20 air", ""].map(String::from).into();
    lines.extend(["summon zombie ~8 ~ ~", "summon cow ~-8 ~ ~"].map(String::from));
    lines.extend(std::iter::repeat_n(String::new(), 60));
    lines.extend(["tp @s @e[type=cow,c=1]", "", "!hit cow"].map(String::from));
    lines.extend(std::iter::repeat_n(String::new(), 10));
    lines.extend(["kill @e[type=!player]", ""].map(String::from));
    lines
}
