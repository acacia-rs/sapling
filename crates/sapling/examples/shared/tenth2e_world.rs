//! tenth2e_drill.rs's world drills: a shown bundle put in a chest and one a fox takes up
//! (`t2ebundle`), a chest boat's goods by the inventory key and vehicles by a blast (`t2eboats`),
//! a head in stone and a bed in the Nether (`t2ebreath`), peaceful on the kinds left undrilled
//! (`t2epeace`), what the world folder keeps of an orb, a baby and a boat (`t2ekeep`), a crossbow's
//! rocket and Channeling on the open ground (`t2efire`), a portal stood in (`t2eportal`), the
//! End's exit taken thrice (`t2ecredits`), Multishot and Loyalty (`t2emulti`).

use super::super::creatures::{floor, waits, with};

const SHOWN: &[&str] = &[
    "AddEntity", "RemoveEntity", "AddItemEntity", "TakeItemEntity", "SetEntityData", "SetEntityMotion", "EntityEvent", "LevelSoundEvent", "LevelEvent", "MobEffect", "UpdateAttributes",
    "SetHealth", "MobEquipment", "InventorySlot", "InventoryContent", "ContainerOpen", "ContainerClose", "ContainerRegistryCleanup", "UpdateBlock", "HurtAnimation", "ChangeDimension",
    "Text", "Explode",
];
const CREDITS: &[&str] = &["ChangeDimension", "ShowCredits", "Respawn", "SetHealth", "PlayStatus", "Text"];
const MOVES: &[&str] = &["AddEntity", "RemoveEntity", "MoveEntityDelta", "SetEntityMotion", "SetEntityData", "LevelSoundEvent", "InventorySlot", "TakeItemEntity"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "t2ebundle" => (bundle(), SHOWN),
        "t2eboats" => (boats(), SHOWN),
        "t2ebreath" => (breath(), SHOWN),
        "t2epeace" => (peace(), SHOWN),
        "t2epeace2" => (peace2(), SHOWN),
        "t2ereload" => (reload(), SHOWN),
        "t2ekeep" => (keep(), SHOWN),
        "t2efire" => (fire(), SHOWN),
        "t2eportal" => (portal(), SHOWN),
        "t2emulti" => (multi(), MOVES),
        "t2ecredits" => (credits(), CREDITS),
        _ => return None,
    })
}

fn held(kind: &str) -> String {
    format!("effect @e[type={kind}] slowness 120 255 true")
}

/// A bundle with cobblestone in it put in a chest and left 34 s; another dropped where a fox
/// held still stands, left 34 s.
fn bundle() -> Vec<String> {
    let mut lines = with(floor(), &["gamemode survival @s", "setblock 2 -26 0 chest", "replaceitem entity @s slot.hotbar 0 bundle 1", "replaceitem entity @s slot.hotbar 1 cobblestone 64", "", "!bundletag", "!bundleput 1 10 0", ""]);
    lines.extend(["!click 2 0 0", "", "!put 0 0", "", "!close", ""].map(String::from));
    waits(&mut lines, 34);
    lines.extend(["setblock 2 -26 0 air", "kill @e[type=item]", "summon fox 2.5 -26 0.5", &held("fox"), "replaceitem entity @s slot.hotbar 0 bundle 1", "", "!bundletag", "!bundleput 1 10 0", ""].map(String::from));
    lines.extend(["!drop 0 1", "", "", "", ""].map(String::from));
    waits(&mut lines, 34);
    with(lines, &["kill @e[type=!player]", "gamemode creative @s", "clear @s"])
}

/// The actor rides a chest boat and presses the inventory key; then boats and carts 2, 4 and 6
/// from a TNT that goes off.
fn boats() -> Vec<String> {
    let mut lines = with(floor(), &["gamemode survival @s", "summon chest_boat 2.5 -26 0.5", "", "!mount chest_boat", "", "!openinv", "", "", "!close", "!dismount", "", "kill @e[type=chest_boat]"]);
    lines.extend(["tp @s -6.5 -26 -6.5", "gamerule tntexplodes true", "summon boat 10.5 -26 0.5", "summon boat 12.5 -26 0.5", "summon boat 14.5 -26 0.5"].map(String::from));
    lines.extend(["summon minecart 8.5 -26 2.5", "summon minecart 8.5 -26 4.5", "summon minecart 8.5 -26 6.5", "", "summon tnt 8.5 -26 0.5"].map(String::from));
    waits(&mut lines, 7);
    with(lines, &["kill @e[type=!player]", "fill -8 -27 -8 16 -27 8 stone", "fill -8 -26 -8 16 -16 8 air", "gamemode creative @s"])
}

/// The survival actor's head in stone 15 s, its feet in air; then a bed in the Nether slept in.
fn breath() -> Vec<String> {
    let mut lines = with(floor(), &["gamemode survival @s", "effect @s health_boost 300 9 true", "effect @s instant_health 1 9 true", "setblock 4 -25 0 stone", "tp @s 4.5 -26 0.5"]);
    waits(&mut lines, 15);
    lines.extend(["setblock 4 -25 0 air", "tp @s 0.5 -26 0.5", "effect @s instant_health 1 9 true", ""].map(String::from));
    lines.extend(["execute in nether run tp @s 0.5 100 0.5", "", "", "", "fill -3 99 -3 4 99 3 obsidian", "fill -3 100 -3 4 104 3 air", "setblock 2 100 0 bed [\"direction\"=1]", "tp @s 0.5 100 0.5", "", "", "!sleep 2 0 0"].map(String::from));
    waits(&mut lines, 3);
    with(lines, &["gamemode creative @s", "execute in overworld run tp @s 0.5 -26 0.5", "", "", "effect @s clear"])
}

/// Kinds the earlier peaceful drill lost before the switch, under a glass roof at midnight,
/// held still and spared fire; then peaceful.
fn peace() -> Vec<String> {
    let mut lines = with(floor(), &["time set midnight", "fill -8 -22 -8 16 -22 8 glass"]);
    let kinds = [
        "blaze", "piglin", "silverfish", "zoglin", "zombie_pigman", "zombie_villager_v2", "hoglin", "piglin_brute", "endermite", "magma_cube", "slime", "phantom", "vex", "breeze", "bogged",
        "wither_skeleton", "stray", "husk", "witch", "pillager", "vindicator", "evoker", "ravager", "spider", "cave_spider", "enderman", "creeper", "skeleton", "drowned", "zombie",
    ];
    for (n, kind) in kinds.iter().enumerate() {
        let (x, z) = ((n % 10) as i32 * 2 - 6, (n / 10) as i32 * 4 - 5);
        lines.push(format!("summon {kind} {x}.5 -26 {z}.5"));
    }
    lines.extend(["effect @e[type=!player] slowness 120 255 true", "effect @e[type=!player] fire_resistance 120 0 true", "", "", "difficulty peaceful"].map(String::from));
    waits(&mut lines, 5);
    with(lines, &["difficulty easy", "kill @e[type=!player]", "fill -8 -22 -8 16 -22 8 air"])
}

/// The kinds `t2epeace` still lost before the switch (piglins turn in the overworld, others
/// fought), summoned apart, unhurtable, and peaceful at once.
fn peace2() -> Vec<String> {
    let mut lines = with(floor(), &["time set midnight", "fill -8 -22 -8 16 -22 8 glass"]);
    for (n, kind) in ["piglin", "hoglin", "piglin_brute", "silverfish", "witch", "evocation_illager", "zombie_villager"].iter().enumerate() {
        lines.push(format!("summon {kind} {}.5 -26 -5.5", n as i32 * 3 - 6));
    }
    lines.extend(["effect @e[type=!player] resistance 120 255 true", "effect @e[type=!player] slowness 120 255 true", "difficulty peaceful"].map(String::from));
    waits(&mut lines, 3);
    with(lines, &["difficulty easy", "kill @e[type=!player]", "fill -8 -22 -8 16 -22 8 air"])
}

/// A cow held still, left behind 2000 off until its column unloads, then come back to: what its
/// loading from the world folder is announced with.
fn reload() -> Vec<String> {
    let mut lines = with(floor(), &["summon cow 3.5 -26 3.5", &held("cow"), "effect @a slow_falling 60 0 true", "", "tp @a 2000.5 -20 2000.5", "", "", "fill 1998 -27 1998 2002 -27 2002 stone"]);
    waits(&mut lines, 30);
    lines.extend(["tp @a 0.5 -26 0.5", "", "", "", "", "kill @e[type=cow]", "effect @a clear"].map(String::from));
    lines
}

/// Left in the world when the bots go, for `actor_dump`: an orb worth 7, a baby cow, a boat.
fn keep() -> Vec<String> {
    with(floor(), &["summon xp_orb 3.5 -26 -20.5", "summon cow 6.5 -26 -3.5 0 0 minecraft:entity_born", &held("cow"), "summon boat 9.5 -26 -3.5", "", "", ""])
}

/// A Multishot crossbow loaded and shot east; a Loyalty III trident thrown east at a glass wall 8
/// off and left to come back.
fn multi() -> Vec<String> {
    let mut lines = with(floor(), &["gamemode survival @s", "tp @s -7.5 -26 0.5 -90 0", "replaceitem entity @s slot.weapon.mainhand 0 crossbow", "enchant @s multishot 1", "replaceitem entity @s slot.inventory 0 arrow 8", ""]);
    lines.extend(["!use", "", "", "!release", "", "!use", "", "", "kill @e[type=arrow]"].map(String::from));
    lines.extend(["fill 1 -26 -3 1 -20 3 glass", "replaceitem entity @s slot.weapon.mainhand 0 trident", "enchant @s loyalty 3", "", "!use", "", "!release"].map(String::from));
    waits(&mut lines, 6);
    with(lines, &["fill 1 -26 -3 1 -20 3 air", "kill @e[type=thrown_trident]", "gamemode creative @s", "clear @s"])
}

/// A Nether portal stood in 10 s by a spectator, then in survival until it goes through.
fn portal() -> Vec<String> {
    let mut lines = with(floor(), &["fill 3 -27 -1 3 -22 2 obsidian", "fill 3 -26 0 3 -23 1 portal [\"portal_axis\"=\"z\"]", "gamemode spectator @s", "tp @s 3.5 -26 0.5"]);
    waits(&mut lines, 10);
    lines.extend(["tp @s 0.5 -26 0.5", "gamemode survival @s", "", "tp @s 3.5 -26 0.5"].map(String::from));
    waits(&mut lines, 8);
    with(lines, &["gamemode creative @s", "execute in overworld run tp @s 0.5 -26 0.5", "", "", "fill 3 -27 -1 3 -22 2 air"])
}

/// An exit portal block in the End stepped into three times, the last after a rejoin: when the
/// credits show.
fn credits() -> Vec<String> {
    let end = |command: &str| format!("execute in the_end run {command}");
    // The End's chunks must be loaded before the fills: the first run's fills did nothing.
    let mut lines = with(floor(), &["effect @s slow_falling 10 0 true", &end("tp @s 98.5 70 0.5"), "", ""]);
    lines.extend([end("fill 98 59 -2 102 59 2 obsidian"), end("fill 98 60 -2 102 63 2 air"), end("setblock 100 60 0 end_portal"), "effect @s clear".into(), "gamemode survival @s".into()]);
    for rejoin in [false, false, true] {
        if rejoin {
            lines.extend(["!rejoin", "", "", ""].map(String::from));
        }
        lines.extend([end("tp @s 98.5 60 0.5"), String::new(), String::new(), "tp @s 100.5 60 0.5".into()]);
        waits(&mut lines, 8);
    }
    with(lines, &["gamemode creative @s", "execute in overworld run tp @s 0.5 -26 0.5", ""])
}

/// On the open ground: a crossbow loaded with a rocket in the offhand and shot; in thunder, a
/// Channeling trident thrown at a cow held still 6 east.
fn fire() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "gamemode survival @s", "clear @s", "difficulty easy", "time set noon", "weather clear", "tp @s 0.5 -60 0.5 -90 0", "tp CmdWatcher 0.5 -60 3.5", "", ""]
        .map(String::from)
        .into();
    lines.extend(["replaceitem entity @s slot.weapon.mainhand 0 crossbow", "replaceitem entity @s slot.weapon.offhand 0 firework_rocket 4", "", "!use", "", "", "!release", "", "!use", "", "", ""].map(String::from));
    lines.extend(["summon cow 6.5 -60 0.5", &held("cow"), "effect @e[type=cow] health_boost 60 4 true", "weather thunder", "", "replaceitem entity @s slot.weapon.mainhand 0 trident", "enchant @s channeling 1"].map(String::from));
    lines.extend(["!look 6 0.8 0", "", "!use", "", "!release"].map(String::from));
    waits(&mut lines, 5);
    with(lines, &["weather clear", "kill @e[type=!player]", "gamemode creative @s", "clear @s"])
}
