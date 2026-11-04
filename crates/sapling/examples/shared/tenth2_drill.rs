//! The tenth wave's mob drills (research/parity/TENTH-BRIEF.md "Group A"), on creature_drill.rs's floor
//! (stone at y -27) or harm_drill.rs's wider one: effect flags on mobs (`t2flags`), a zombie pigman in
//! fire, lava, a fall and water (`t2pigman`), babies' experience and mobs once hit (`t2babyxp`), a
//! hitter's hunger (`t2hunger`), bartering by click (`t2barter`), guardians and squid, a cod ashore
//! (`t2water`), the wither's fire rate, dash and skulls at mobs (`t2wither`), a shrieker that can
//! summon (`t2warden`), two catalysts and one cell's cursors (`t2catalysts`), monsters in a storm by
//! day (`t2storm`, the open world).

use super::creatures::{floor, waits, with};

const SHOWN: &[&str] = &[
    "AddEntity", "RemoveEntity", "AddItemEntity", "MoveEntityDelta", "SetEntityMotion", "SetEntityData", "EntityEvent", "LevelSoundEvent", "LevelEvent", "LevelEventGeneric",
    "MobEffect", "UpdateAttributes", "SetHealth", "MobEquipment", "UpdateBlock", "UpdateSubchunkBlocks", "TakeItemEntity", "InventorySlot",
];
const SPAWNS: &[&str] = &["AddEntity", "RemoveEntity", "LevelEvent"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "t2flags" => (flags(), SHOWN),
        "t2pigman" => (pigman(), SHOWN),
        "t2babyxp" => (baby_xp(), SHOWN),
        "t2hunger" => (hunger(), SHOWN),
        "t2barter" => (barter(), SHOWN),
        "t2water" => (water(), SHOWN),
        "t2wither" => (wither(), SHOWN),
        "t2warden" => (warden(), SHOWN),
        "t2catalysts" => (catalysts(), SHOWN),
        "t2storm" => (storm(), SPAWNS),
        _ => return None,
    })
}

fn held(kind: &str) -> String {
    format!("effect @e[type={kind}] slowness 120 255 true")
}

/// A cow and a zombie given fire resistance, water breathing and speed in turn, then cleared.
fn flags() -> Vec<String> {
    let mut lines = with(floor(), &["time set midnight", "summon cow 4.5 -26 0.5", "summon zombie 4.5 -26 4.5", "effect @e[type=zombie] fire_resistance 60 0 true", &held("cow"), &held("zombie")]);
    waits(&mut lines, 2);
    for effect in ["fire_resistance", "water_breathing", "speed"] {
        lines.push(format!("effect @e[type=cow] {effect} 30 1"));
        waits(&mut lines, 2);
    }
    lines.extend(["effect @e[type=cow] clear", "", "", "effect @e[type=zombie] water_breathing 30 0", "", "", "effect @e[type=zombie] clear", "", ""].map(String::from));
    lines
}

/// Zombie pigmen held still: in a lava hole, in fire on netherrack, dropped 10, and under water,
/// a zombie beside each but the fall for the difference.
fn pigman() -> Vec<String> {
    let mut lines = with(floor(), &[
        "time set midnight",
        "fill 7 -28 -1 9 -28 1 stone",
        "setblock 8 -27 0 lava",
        "setblock 4 -27 4 netherrack",
        "setblock 4 -26 4 fire",
        "fill -5 -28 -5 -3 -21 -3 glass",
        "fill -4 -27 -4 -4 -21 -4 water",
        "fill 11 -28 -1 13 -28 1 stone",
        "setblock 12 -27 0 lava",
        "setblock 8 -27 4 netherrack",
        "setblock 8 -26 4 fire",
        "fill -1 -28 -5 1 -21 -3 glass",
        "fill 0 -27 -4 0 -21 -4 water",
    ]);
    lines.extend(["summon zombie_pigman 8.5 -27 0.5", "summon zombie_pigman 4.5 -26 4.5", "summon zombie_pigman 0.5 -16 4.5", "summon zombie_pigman -3.5 -27 -3.5"].map(String::from));
    lines.extend(["summon zombie 12.5 -27 0.5", "summon zombie 8.5 -26 4.5", "summon zombie 0.5 -27 -3.5", &held("zombie_pigman"), &held("zombie")].map(String::from));
    waits(&mut lines, 25);
    with(lines, &["kill @e[type=!player]", "fill -8 -27 -8 16 -27 8 stone", "fill -8 -26 -8 16 -16 8 air", ""])
}

/// Killed by the survival actor's strong hits: a grown cow, a baby cow, a baby zombie, a baby pig;
/// then a cow and a zombie hit once with a fist and watched.
fn baby_xp() -> Vec<String> {
    let mut lines = with(floor(), &["time set midnight", "gamemode survival @s", "effect @s resistance 120 4 true", "effect @s strength 60 20 true", "replaceitem entity @s slot.weapon.mainhand 0 netherite_sword"]);
    for summon in ["cow 2.5 -26 0.5", "cow 2.5 -26 0.5 0 0 minecraft:entity_born", "zombie 2.5 -26 0.5 0 0 minecraft:as_baby", "pig 2.5 -26 0.5 0 0 minecraft:entity_born"] {
        let kind = summon.split(' ').next().unwrap_or_default();
        lines.extend([format!("summon {summon}"), held(kind), format!("!hit {kind}")]);
        waits(&mut lines, 3);
        lines.push("kill @e[type=xp_orb]".into());
    }
    lines.extend(["kill @e[type=!player]", "effect @s clear", "effect @s resistance 120 4 true", "replaceitem entity @s slot.weapon.mainhand 0 air", "summon cow 2.5 -26 0.5", "", "!hit cow"].map(String::from));
    waits(&mut lines, 15);
    lines.extend(["summon zombie 2.5 -26 3.5", "", "!hit zombie"].map(String::from));
    waits(&mut lines, 10);
    with(lines, &["kill @e[type=!player]", "gamemode creative @s"])
}

/// The survival actor, fed full, hits a cow held still and kept alive 40 times with a fist.
fn hunger() -> Vec<String> {
    let mut lines = with(floor(), &["gamemode survival @s", "effect @s resistance 120 4 true", "effect @s saturation 1 255 true", "summon cow 2.0 -26 0.5", &held("cow"), "effect @e[type=cow] regeneration 120 255 true", "", ""]);
    lines.extend(std::iter::repeat_n("!hit cow".to_owned(), 40));
    waits(&mut lines, 2);
    with(lines, &["kill @e[type=!player]", "gamemode creative @s"])
}

/// A piglin held by the actor clicked with gold, and one hit first and then clicked; watched until
/// both have handed back (the overworld turns them 15 s on).
fn barter() -> Vec<String> {
    let mut lines = with(floor(), &["replaceitem entity @s slot.weapon.mainhand 0 gold_ingot 8", "summon piglin 2.0 -26 0.5", &held("piglin"), "", "!interact piglin", "summon piglin 2.0 -26 1.5", &held("piglin"), "", "!hit piglin", "!interact piglin"]);
    waits(&mut lines, 14);
    with(lines, &["kill @e[type=!player]", "replaceitem entity @s slot.weapon.mainhand 0 air"])
}

/// A pool with a guardian, a squid and an axolotl, the actor in creative out of it; a cod on dry stone.
fn water() -> Vec<String> {
    let mut lines = with(floor(), &["time set midnight", "fill 3 -27 -6 14 -21 6 water", "summon cod -4.5 -26 -4.5", "summon guardian 8.5 -25 0.5", "summon squid 12.5 -24 4.5", "summon axolotl 12.5 -26 -4.5"]);
    waits(&mut lines, 30);
    with(lines, &["kill @e[type=!player]", "fill 3 -27 -6 14 -21 6 air", "fill -8 -27 -8 16 -27 8 stone", ""])
}

/// A summoned wither past its charge at the survival actor (resisting): its skulls for 12 s at full
/// health and after each sixth lost; then armoured, its dash at the actor unwarded (more hearts,
/// healed); then the actor out of play and a cow to shoot at.
fn wither() -> Vec<String> {
    let mut lines = super::warden::start("noon");
    lines.extend(["gamerule mobgriefing false", "summon wither 0.5 -26 -12.5"].map(String::from));
    waits(&mut lines, 12);
    lines.push("effect @s resistance 200 4 true".into());
    for _ in 0..3 {
        waits(&mut lines, 12);
        lines.push("damage @e[type=wither] 50".into());
    }
    waits(&mut lines, 6);
    lines.extend(["damage @e[type=wither] 60", "effect @s clear", "effect @s health_boost 900 9 true"].map(String::from));
    super::warden::healed(&mut lines, 40);
    lines.extend(["gamemode creative @s", "summon cow 0.5 -26 6.5", &held("cow"), "effect @e[type=cow] health_boost 60 4 true", "effect @e[type=cow] instant_health 1 9 true"].map(String::from));
    waits(&mut lines, 20);
    with(lines, &["kill @e[type=!player]", "gamerule mobgriefing true"])
}

/// A shrieker that can summon under the survival actor, which stands on it a minute.
fn warden() -> Vec<String> {
    let mut lines = super::warden::start("midnight");
    lines.extend(["setblock 0 -27 4 sculk_shrieker [\"can_summon\"=true]", "tp @s 0.5 -26 4.5"].map(String::from));
    super::warden::healed(&mut lines, 60);
    with(lines, &["gamemode creative @s", "kill @e[type=!player]", "setblock 0 -27 4 stone"])
}

/// Catalysts at 0 -27 -8 and 6 -27 -8: a cow killed 2 off the east one, then 2 off the west one, then
/// one midway; two cows killed on one cell; the spread left to run.
fn catalysts() -> Vec<String> {
    let mut lines = super::harm::start("midnight", 4);
    lines.extend(["setblock 0 -27 -8 sculk_catalyst", "setblock 6 -27 -8 sculk_catalyst", ""].map(String::from));
    for summon in ["cow 4.5 -26 -7.5", "cow 2.5 -26 -7.5", "cow 3.5 -26 -7.5"] {
        lines.extend([format!("summon {summon}"), String::new(), "kill @e[type=cow]".into()]);
        waits(&mut lines, 4);
    }
    lines.extend(["summon cow 3.5 -26 -11.5", "summon cow 3.5 -26 -11.5", "", "kill @e[type=cow]"].map(String::from));
    waits(&mut lines, 12);
    with(lines, &["kill @e[type=!player]", "fill -12 -27 -20 12 -27 4 stone", ""])
}

/// The open world's ground at noon: monsters come in 30 s of clear sky, then in 60 s of thunder.
fn storm() -> Vec<String> {
    let mut lines: Vec<String> = [
        "kill @e[type=!player]",
        "gamemode creative @s",
        "gamemode creative CmdWatcher",
        "difficulty easy",
        "gamerule dodaylightcycle false",
        "time set noon",
        "weather clear",
        "tp @s 0.5 -60 0.5",
        "tp CmdWatcher 0.5 -60 0.5",
        "gamerule domobspawning true",
    ]
    .map(String::from)
    .into();
    waits(&mut lines, 30);
    lines.push("weather thunder".into());
    waits(&mut lines, 60);
    with(lines, &["gamerule domobspawning false", "weather clear", "kill @e[type=!player]", "gamerule dodaylightcycle true"])
}
