//! The drills of what the combat drills left: a mace's smash and its enchantments, powder snow's
//! freezing, Soul Speed, Swift Sneak and Aqua Affinity, and a ridden horse's charged jump.

use acacia_bot::Bot;

/// The actor in survival on a wide floor, away from the watcher.
const FLOOR: [&str; 8] = ["kill @e[type=!player]", "gamerule domobspawning false", "difficulty easy", "fill ~-3 ~-1 ~-12 ~12 ~-1 ~40 stone", "fill ~-3 ~ ~-12 ~12 ~16 ~40 air", "gamemode survival @s", "tp @s ~6 ~ ~20", ""];
const HEAL: &str = "effect @s instant_health 1 4";
/// Packets the drills here show: a hit's, a freeze's and a break's.
pub const SHOWN: &[&str] = &[
    "UpdateAttributes", "EntityEvent", "SetEntityData", "LevelSoundEvent", "LevelEvent", "LevelEventGeneric", "SetEntityMotion", "Animate", "HurtAnimation", "AddEntity", "RemoveEntity",
    "MoveEntityDelta", "MoveEntityAbsolute", "MovePlayer", "InventoryContent", "InventorySlot", "MobArmorEquipment", "MobEquipment", "SetHealth", "UpdateBlock", "SpawnParticleEffect",
    "CorrectPlayerMovePrediction", "MotionPredictionHints", "DeathInfo",
];

fn lines(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|line| line.to_string()).collect()
}

fn wait(lines: &mut Vec<String>, seconds: usize) {
    lines.extend(std::iter::repeat_n(String::new(), seconds));
}

/// A fresh iron golem (no throw, no armour) two east of the actor, penned in barriers, and cows
/// standing loose 1.5, 2.5, 3.5 and 5 from it.
fn target(lines: &mut Vec<String>, kind: &str) {
    lines.extend(["kill @e[type=!player]", "fill ~1 ~ ~-1 ~3 ~2 ~1 barrier", "fill ~2 ~ ~ ~2 ~2 ~ air"].map(String::from));
    lines.push(format!("summon {kind} ~2 ~ ~"));
    lines.extend(["summon cow ~2 ~ ~-1.5", "summon cow ~2 ~ ~2.5", "summon cow ~5.5 ~ ~", "summon cow ~7 ~ ~", HEAL, ""].map(String::from));
}

/// The actor with a mace smashes a penned golem from a little higher each time, hitting as its
/// feet pass 2.2 over the floor (nearer is out of the bot's reach), then with Density 3, then a zombie in a diamond chestplate with and without
/// Breach 4, then with Wind Burst 1; each landing shows whether the fall still hurts.
pub fn mace() -> Vec<String> {
    let mut lines = lines(&FLOOR);
    lines.extend(["clear @s", "replaceitem entity @s slot.weapon.mainhand 0 mace", ""].map(String::from));
    for up in [2.5, 3.5, 4.0, 5.0, 8.0, 13.0, 17.0] {
        target(&mut lines, "iron_golem");
        lines.extend([format!("!smash iron_golem {up} 2.2"), String::new(), String::new()]);
    }
    lines.extend(["enchant @s density 3".into(), String::new()]);
    target(&mut lines, "iron_golem");
    lines.extend(["!smash iron_golem 8 2.2".into(), String::new(), String::new()]);
    for breach in [false, true] {
        lines.extend(["replaceitem entity @s slot.weapon.mainhand 0 mace".into(), String::new()]);
        if breach {
            lines.extend(["enchant @s breach 4".into(), String::new()]);
        }
        target(&mut lines, "zombie");
        lines.extend(["replaceitem entity @e[type=zombie] slot.armor.chest 0 diamond_chestplate".into(), String::new(), "!smash zombie 6 2.2".into(), String::new(), String::new()]);
    }
    lines.extend(["replaceitem entity @s slot.weapon.mainhand 0 mace".into(), "enchant @s wind_burst 1".into(), String::new()]);
    target(&mut lines, "iron_golem");
    lines.extend(["!smash iron_golem 8 2.2".into(), String::new(), String::new(), String::new()]);
    lines.extend(["kill @e[type=!player]", "gamemode creative @s", "clear @s", ""].map(String::from));
    lines
}

/// The smash's push on mobs around, from falls under and over 5 and with Wind Burst II and III:
/// slowed cows stand 2, 2.5, 3, 3.54, 4.03 and 4.2 from the penned golem. Then plain hits on a
/// zombie in a diamond chestplate without and with Breach 4.
pub fn mace_push() -> Vec<String> {
    let mut lines = lines(&FLOOR);
    for (up, burst) in [(4, 0), (8, 0), (8, 2), (8, 3)] {
        lines.extend(["clear @s", "replaceitem entity @s slot.weapon.mainhand 0 mace", "kill @e[type=!player]", "fill ~1 ~ ~-1 ~3 ~2 ~1 barrier", "fill ~2 ~ ~ ~2 ~2 ~ air", "summon iron_golem ~2 ~ ~"].map(String::from));
        if burst > 0 {
            lines.push(format!("enchant @s wind_burst {burst}"));
        }
        let cows = ["~4 ~ ~", "~2 ~ ~2.5", "~2 ~ ~-3", "~4.5 ~ ~2.5", "~5.5 ~ ~-2", "~2 ~ ~4.2"];
        lines.extend(cows.iter().map(|at| format!("summon cow {at}")));
        lines.extend(["effect @e[type=cow] slowness 60 255 true", HEAL, "", ""].map(String::from));
        lines.extend([format!("!smash iron_golem {up} 2.2"), String::new(), String::new(), String::new()]);
    }
    breach(&mut lines, &[0, 4]);
    lines
}

/// Plain mace hits on a zombie in a diamond chestplate with Breach at each of `levels`.
fn breach(lines: &mut Vec<String>, levels: &[u8]) {
    for level in levels {
        lines.extend(["clear @s", "replaceitem entity @s slot.weapon.mainhand 0 mace", "kill @e[type=!player]", "fill ~1 ~ ~-1 ~3 ~2 ~1 barrier", "fill ~2 ~ ~ ~2 ~2 ~ air", "summon zombie ~2 ~ ~"].map(String::from));
        if *level > 0 {
            lines.push(format!("enchant @s breach {level}"));
        }
        lines.extend(["replaceitem entity @e[type=zombie] slot.armor.chest 0 diamond_chestplate", "!look 2 1 0", "", "!hit zombie", "", ""].map(String::from));
    }
    lines.extend(["kill @e[type=!player]", "gamemode creative @s", "clear @s", ""].map(String::from));
}

/// Breach I to III, after the IV of `mace_push`.
pub fn mace_breach() -> Vec<String> {
    let mut lines = lines(&FLOOR);
    breach(&mut lines, &[1, 2, 3]);
    lines
}

/// The actor stands in powder snow filling its own two cells: bare for 16 s, then out of it for 5,
/// then in a leather helmet, then in leather boots, 10 s each.
pub fn freeze() -> Vec<String> {
    let mut lines = lines(&FLOOR);
    let snow = |lines: &mut Vec<String>, seconds: usize| {
        lines.extend(["fill ~ ~ ~ ~ ~1 ~ powder_snow"].map(String::from));
        wait(lines, seconds);
        lines.extend(["fill ~ ~ ~ ~ ~1 ~ air"].map(String::from));
        wait(lines, 5);
        lines.push(HEAL.into());
    };
    lines.extend(["clear @s", HEAL, ""].map(String::from));
    snow(&mut lines, 16);
    lines.push("replaceitem entity @s slot.armor.head 0 leather_helmet".into());
    snow(&mut lines, 10);
    lines.extend(["replaceitem entity @s slot.armor.head 0 air", "replaceitem entity @s slot.armor.feet 0 leather_boots"].map(String::from));
    snow(&mut lines, 10);
    lines.extend(["gamemode creative @s", "clear @s", ""].map(String::from));
    lines
}

/// Walks along soul sand and then soul soil bare-footed and in Soul Speed 3 boots, sneaks in Swift
/// Sneak 3 leggings and without, then, under water, breaks dirt by hand bare-headed and in an Aqua
/// Affinity helmet, and once more on dry land.
pub fn soles() -> Vec<String> {
    let mut lines = lines(&FLOOR);
    lines.extend(["clear @s", HEAL, "fill ~ ~-1 ~-1 ~ ~-1 ~-8 soul_sand", "fill ~ ~-1 ~-9 ~ ~-1 ~-14 soul_soil", "!look 0 1.6 -6", ""].map(String::from));
    for boots in ["air", "diamond_boots"] {
        lines.push(format!("replaceitem entity @s slot.armor.feet 0 {boots}"));
        if boots != "air" {
            lines.extend(["replaceitem entity @s slot.weapon.mainhand 0 diamond_boots".into(), "enchant @s soul_speed 3".into(), "!use".into()]);
        }
        lines.extend(["", "!walk", "", "", "", "!stop", "tp @s ~ ~ ~13", ""].map(String::from));
    }
    lines.extend(["replaceitem entity @s slot.armor.feet 0 air", "fill ~ ~-1 ~-1 ~ ~-1 ~-14 stone", ""].map(String::from));
    for legs in [false, true] {
        if legs {
            lines.extend(["replaceitem entity @s slot.weapon.mainhand 0 diamond_leggings".into(), "enchant @s swift_sneak 3".into(), "!use".into()]);
        }
        lines.extend(["!sneak", "", "!walk", "", "", "!stop", "!unsneak", "tp @s ~ ~ ~4", ""].map(String::from));
    }
    lines.extend(["clear @s", "fill ~-2 ~-1 ~-2 ~2 ~-1 ~2 stone", "fill ~-1 ~ ~-1 ~1 ~2 ~1 water", "setblock ~ ~ ~-1 dirt", "setblock ~ ~1 ~-1 dirt", "!look 0 0.5 -1", ""].map(String::from));
    for helmet in [false, true] {
        if helmet {
            lines.extend(["replaceitem entity @s slot.weapon.mainhand 0 diamond_helmet".into(), "enchant @s aqua_affinity 1".into(), "!use".into(), "setblock ~ ~ ~-1 dirt".into()]);
        }
        lines.extend(["!break 0 0 -1", "", ""].map(String::from));
    }
    lines.extend(["fill ~-1 ~ ~-1 ~1 ~2 ~1 air", "setblock ~ ~ ~-1 dirt", "", "!break 0 0 -1", "", "gamemode creative @s", "clear @s", ""].map(String::from));
    lines
}

/// A tamed, saddled horse ridden by the actor: it walks, then holds jump 3, 8 and 15 ticks.
pub fn horse_jump() -> Vec<String> {
    let mut lines = lines(&FLOOR);
    lines.extend((0..8).map(|n| format!("summon horse ~{} ~ ~10", n * 2 - 7)));
    lines.extend(["", "kill @e[type=horse]", "clear @s", "summon horse ~ ~ ~-3 0 0 minecraft:on_tame", "","replaceitem entity @e[type=horse] slot.saddle 0 saddle", "", "!mount horse", ""].map(String::from));
    lines.extend(["!look 0 1.6 -8", "!walk", "", "!stop", ""].map(String::from));
    for ticks in [3, 8, 15] {
        lines.extend([format!("!charge {ticks}"), String::new(), String::new()]);
    }
    lines.extend(["!dismount", "kill @e[type=!player]", "gamemode creative @s", ""].map(String::from));
    lines
}

/// The lines of the drill `name`, if it is one of these.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some((
        match name {
            "mace" => mace(),
            "mace-push" => mace_push(),
            "mace-breach" => mace_breach(),
            "freeze" => freeze(),
            "soles" => soles(),
            "horse-jump" => horse_jump(),
            _ => return None,
        },
        SHOWN,
    ))
}

fn feet_y(actor: &Bot) -> Option<f32> {
    actor.movement().and_then(|movement| movement.position()).map(|at| at[1])
}

fn newest(actor: &Bot, kind: &str) -> Option<u64> {
    let kind = format!("minecraft:{kind}");
    actor.state().entities.iter().filter(|entity| entity.kind == kind).map(|entity| entity.runtime_id).max()
}

/// `!smash kind up at`: the actor is put `up` blocks higher and hits the newest `kind` it knows as
/// its feet come back within `at` of where they were; `!charge ticks` holds jump that long on what
/// it rides; `!mount kind` and `!dismount`. False for any other line.
pub async fn act(actor: &mut Bot, line: &str) -> bool {
    if let Some(&[kind, up, at]) = line.strip_prefix("!smash ").map(|words| words.split(' ').collect::<Vec<_>>()).as_deref() {
        let (Some(ground), Ok(up), Ok(at), Some(id)) = (feet_y(actor), up.parse::<f32>(), at.parse::<f32>(), newest(actor, kind)) else {
            println!("## no {kind} to smash");
            return true;
        };
        // Aimed from the floor, the hit goes out at once mid-fall: the bot's own look and hover
        // pause would land it after the landing.
        let target = actor.state().entities.iter().find(|entity| entity.runtime_id == id).map(|entity| [entity.position.x, entity.position.y + 1.0, entity.position.z]);
        if let Some(target) = target {
            let _ = actor.look_at(target).await;
        }
        drop(actor.client().command(&format!("tp @s ~ ~{up} ~")));
        for _ in 0..40 {
            let _ = actor.wait_ticks(1).await;
            if feet_y(actor).is_some_and(|y| y > ground + up - 0.5) {
                break;
            }
        }
        for _ in 0..80 {
            let _ = actor.wait_ticks(1).await;
            if feet_y(actor).is_some_and(|y| y < ground + at) {
                break;
            }
        }
        let at = feet_y(actor).map(|y| y - ground);
        println!("## smash {id} from {up} at {at:?}: {:?}", actor.click_entity_aimed(id, true));
        return true;
    }
    if let Some(ticks) = line.strip_prefix("!charge ").and_then(|ticks| ticks.parse().ok()) {
        if let Some(controls) = actor.controls() {
            controls.jump = true;
        }
        let _ = actor.wait_ticks(ticks).await;
        if let Some(controls) = actor.controls() {
            controls.jump = false;
        }
        return true;
    }
    if let Some(kind) = line.strip_prefix("!mount ") {
        match newest(actor, kind) {
            Some(id) => println!("## mounted {id}: {:?}", actor.mount(id).await.map(|_| ())),
            None => println!("## no {kind} to mount"),
        }
        return true;
    }
    if line == "!dismount" {
        println!("## dismounted: {:?}", actor.dismount().await);
        return true;
    }
    false
}
