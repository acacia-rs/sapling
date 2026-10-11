//! The hurt, smother, hunger, eat, tire, toil, armour and effects drills of bds_command_probe: a
//! player harmed, killed and risen again (docs/research/player-damage.md, effects.md). A zombie's
//! and what food does besides feeding: hunt_drill.rs.

use std::time::Duration;

use acacia_bot::Bot;

#[path = "hunt_drill.rs"]
mod hunt;
#[path = "shove_drill.rs"]
mod shove;

/// Does what a drill's `!` line asks of the actor: `!eat` holds use on what it holds, `!jump`,
/// `!walk` and `!sprint` hold those keys until `!stop`, `!break x y z` breaks, `!click x y z` clicks and
/// `!place x y z` puts what it holds on the block that far from its feet. False for any other line.
pub async fn act(actor: &mut Bot, line: &str) -> bool {
    let keys = |actor: &mut Bot, forward: f32, sprint: bool, jump: bool| {
        if let Some(controls) = actor.controls() {
            (controls.forward, controls.sprint, controls.jump) = (forward, sprint, jump);
        }
    };
    if let Some((places, offset)) = line.strip_prefix("!click ").map(|offset| (false, offset)).or(line.strip_prefix("!place ").map(|offset| (true, offset))) {
        let offset: Vec<i32> = offset.split(' ').filter_map(|part| part.parse().ok()).collect();
        let feet = actor.movement().and_then(|movement| movement.position()).map(|at| at.map(|c| c.floor() as i32));
        if let (Some(feet), &[x, y, z]) = (feet, &offset[..]) {
            let (at, up) = ([feet[0] + x, feet[1] + y, feet[2] + z], acacia_bot::interact::Face::Up);
            match places {
                true => println!("## placed: {:?}", actor.place_block(at, up).await),
                false => println!("## clicked: {:?}", actor.use_item_on_block(at, up).await),
            }
        }
        return true;
    }
    if let Some(offset) = line.strip_prefix("!break ") {
        let offset: Vec<i32> = offset.split(' ').filter_map(|part| part.parse().ok()).collect();
        let feet = actor.movement().and_then(|movement| movement.position()).map(|at| at.map(|c| c.floor() as i32));
        if let (Some(feet), &[x, y, z]) = (feet, &offset[..]) {
            println!("## broke: {:?}", actor.break_block([feet[0] + x, feet[1] + y, feet[2] + z]).await);
        }
        return true;
    }
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

pub const CLOCK_SHOWN: &[&str] = &["UpdateAttributes", "SyncWorldClocks", "SetTime", "Text", "LevelSoundEvent", "LevelEvent"];
pub const SHOOT_SHOWN: &[&str] = &["UpdateAttributes", "EntityEvent", "SetEntityData", "LevelSoundEvent", "SetEntityMotion", "RemoveEntity", "AddEntity", "MoveEntityDelta", "MobEffect", "MobEquipment"];
pub const HURT_SHOWN: &[&str] = &[
    "UpdateAttributes", "EntityEvent", "SetEntityData", "SetHealth", "Respawn", "LevelSoundEvent", "LevelEvent", "MovePlayer", "SetEntityMotion", "Text", "DeathInfo", "PlayerAction", "Animate",
    "MobEffect", "SetPlayerGameType", "UpdatePlayerGameType", "RemoveEntity", "AddPlayer", "AddItemEntity", "AddEntity", "InventoryContent", "InventorySlot", "MobEquipment", "MobArmorEquipment",
    "SetSpawnPosition", "CorrectPlayerMovePrediction", "PlayStatus", "ChangeDimension", "SpawnExperienceOrb", "CompletedUsingItem", "ItemStackResponse",
];

/// The lines and shown packets of the drill `name`, if it is one of these.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "hurt" => (hurt(), HURT_SHOWN),
        "smother" => (smother(), HURT_SHOWN),
        "hunger" => (hunger(), HURT_SHOWN),
        "eat" => (eat(), HURT_SHOWN),
        "tire" => (tire(), HURT_SHOWN),
        "bite" => (hunt::bite(), hunt::BITE_SHOWN),
        "slain" => (hunt::slain(), hunt::BITE_SHOWN),
        "chase" => (hunt::chase(), hunt::BITE_SHOWN),
        "sick" => (hunt::sick(), HURT_SHOWN),
        "worn" => (hunt::worn(), HURT_SHOWN),
        "kinds" => (hunt::kinds(), hunt::KINDS_SHOWN),
        "stroll" => (hunt::stroll(), hunt::KINDS_SHOWN),
        "flee" => (hunt::flee(), hunt::KINDS_SHOWN),
        "around" => (hunt::around(), hunt::BITE_SHOWN),
        "drink" => (hunt::drink(), HURT_SHOWN),
        "more" => (hunt::more(), HURT_SHOWN),
        "shove" => (shove::shove(), shove::SHOVE_SHOWN),
        "byhand" => (shove::byhand(), shove::BYHAND_SHOWN),
        "bedspawn" => (shove::bedspawn(), shove::BED_SHOWN),
        "boom" => (shove::boom(), shove::BOOM_SHOWN),
        "shoot" => (shove::shoot(), shove::BOOM_SHOWN),
        "wear" => (shove::wear(), shove::WEAR_SHOWN),
        "provoke" => (shove::provoke(), hunt::KINDS_SHOWN),
        "fx" => (shove::fx(), HURT_SHOWN),
        "drink2" => (shove::drink2(), HURT_SHOWN),
        "shoot2" => (shove::shoot2(), SHOOT_SHOWN),
        "golem" => (shove::golem(), SHOOT_SHOWN),
        "clock" => (shove::clock(), CLOCK_SHOWN),
        "sounds" => (shove::sounds(), shove::SOUNDS_SHOWN),
        "loot" => (shove::loot(), shove::LOOT_SHOWN),
        "loot2" => (shove::loot2(), shove::LOOT2_SHOWN),
        "names" => (shove::names(), &[]),
        "toil" => (toil(), HURT_SHOWN),
        "armour" => (armour(), HURT_SHOWN),
        "effects" => (effects(), HURT_SHOWN),
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


/// The actor, in survival: breaks four blocks of dirt and one of stone by hand, hits a cow three
/// times and an armour stand once, and swings at nothing.
pub fn toil() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "", "fill ~-3 ~-1 ~-3 ~8 ~-1 ~8 stone", "fill ~-3 ~ ~-3 ~8 ~6 ~8 air", "gamemode survival @s", ""].map(String::from).into();
    for at in 2..6 {
        lines.extend([format!("setblock ~1 ~ ~{at} dirt"), format!("!break 1 0 {at}"), String::new()]);
    }
    lines.extend(["setblock ~1 ~1 ~ stone", "!break 1 1 0", "", "summon cow ~2 ~ ~"].map(String::from));
    for _ in 0..3 {
        lines.extend(["!hit cow", "", ""].map(String::from));
    }
    lines.extend(["kill @e[type=cow]", "summon armor_stand ~2 ~ ~", "", "!hit armor_stand", "", "damage @s 2", "", "", "gamemode creative @s", "kill @e[type=!player]", ""].map(String::from));
    lines
}

/// The actor, in survival, hit for 5 by `/damage` as an entity's attack: bare, in a leather and an
/// iron chestplate, in all of iron, of diamond and of netherite; then for 2 in the netherite by
/// each other cause.
pub fn armour() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "", "fill ~-3 ~-1 ~-3 ~8 ~-1 ~8 stone", "fill ~-3 ~ ~-3 ~8 ~6 ~8 air", "gamemode survival @s", ""].map(String::from).into();
    let wear = |lines: &mut Vec<String>, part: &str, item: &str| lines.push(format!("replaceitem entity @s slot.armor.{part} 0 {item}"));
    let hit = |lines: &mut Vec<String>, harm: &str| lines.extend([format!("damage @s {harm}"), String::new(), "effect @s instant_health 1 4".into(), String::new()]);
    hit(&mut lines, "5 entity_attack");
    for chestplate in ["leather_chestplate", "iron_chestplate"] {
        wear(&mut lines, "chest", chestplate);
        hit(&mut lines, "5 entity_attack");
    }
    for metal in ["iron", "diamond", "netherite"] {
        for (part, piece) in [("head", "helmet"), ("chest", "chestplate"), ("legs", "leggings"), ("feet", "boots")] {
            wear(&mut lines, part, &format!("{metal}_{piece}"));
        }
        hit(&mut lines, "5 entity_attack");
    }
    for cause in ["", " fall", " fire", " drowning", " suffocation", " lava"] {
        hit(&mut lines, &format!("2{cause}"));
    }
    lines.extend(["gamemode creative @s", "clear @s", ""].map(String::from));
    lines
}

/// The actor, in survival, under `/effect`: healed and harmed at once by two strengths, mended by
/// regeneration at two, poisoned to its last health, withered, made quick, and cleared.
pub fn effects() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "", "fill ~-3 ~-1 ~-3 ~8 ~-1 ~8 stone", "fill ~-3 ~ ~-3 ~8 ~6 ~8 air", "gamemode survival @s", ""].map(String::from).into();
    let wait = |lines: &mut Vec<String>, seconds: usize| lines.extend(std::iter::repeat_n(String::new(), seconds));
    lines.extend(["damage @s 18", "", "effect @s instant_health 1 0", "", "effect @s instant_health 1 1", "", "effect @s instant_health 1 4", ""].map(String::from));
    lines.extend(["effect @s instant_damage 1 0", "", "effect @s instant_health 1 4", "", "effect @s instant_damage 1 1", "", "damage @s 4", ""].map(String::from));
    lines.push("effect @s regeneration 6 0".into());
    wait(&mut lines, 8);
    lines.push("effect @s regeneration 3 2 true".into());
    wait(&mut lines, 5);
    lines.extend(["effect @s instant_health 1 4", "", "effect @s poison 30 0"].map(String::from));
    wait(&mut lines, 28);
    lines.extend(["effect @s clear", "", "effect @s instant_health 1 4", "", "effect @s poison 3 1"].map(String::from));
    wait(&mut lines, 5);
    lines.extend(["effect @s instant_health 1 4", "", "effect @s wither 5 0"].map(String::from));
    wait(&mut lines, 7);
    lines.extend(["effect @s instant_health 1 4", "", "effect @s speed 3 1", "", "effect @s speed 3 0", "", "effect @s speed 0", "", "effect @s nosuch 3 1", "effect @s regeneration", "effect @s regeneration 5 300", "effect @s clear", "effect @s clear", ""].map(String::from));
    lines.extend(["gamemode creative @s", "effect @s poison 3 1", "", "", "", "", ""].map(String::from));
    lines
}
