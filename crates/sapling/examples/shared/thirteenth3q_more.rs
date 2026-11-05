//! More group Q scenes (thirteenth3q_drill.rs's floor). `q3keep` leaves mobs in the world and `q3kept`
//! joins it again (`KEEP_WORLD=1`: what a mob loaded from the folder is first sent). `q3crowd`: held cows
//! and medium slimes beside the still actor at box gaps 0.05 to 0.3. `q3wind`: a wind charge into a
//! wall with held cows 1 to 4 aside. `q3love`: a lone cow fed (how long its love lasts), then four
//! pairs fed (their babies' orbs).

use crate::creatures::{waits, with};
use crate::warden::start;

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

pub fn keep() -> Vec<String> {
    let mut lines = with(start("noon"), &["gamemode creative @s", "summon cow 3.5 -26 3.5", "summon sheep -3.5 -26 3.5", "summon husk 3.5 -26 -3.5", "summon pig -3.5 -26 -3.5"]);
    lines.extend(owned(["effect @e[type=!player] slowness 600 255 true", "replaceitem entity @e[type=husk] slot.armor.head 0 iron_helmet"]));
    waits(&mut lines, 4);
    with(lines, &["testfor @e[type=!player]"])
}

pub fn kept() -> Vec<String> {
    let mut lines = owned(["", "testfor @e[type=!player]"]);
    waits(&mut lines, 4);
    with(lines, &["kill @e[type=!player]"])
}

pub fn crowd() -> Vec<String> {
    let mut lines = with(start("noon"), &["difficulty easy", "effect @s resistance 900 4 true", "tp @s 0.5 -26 0.5 0 0"]);
    for gap in ["1.3", "1.4", "1.5", "1.55"] {
        lines.extend([format!("summon cow {gap} -26 0.5"), "effect @e[type=cow] slowness 60 255 true".into()]);
        waits(&mut lines, 3);
        lines.push("kill @e[type=cow]".into());
    }
    for gap in ["1.42", "1.52", "1.62"] {
        lines.extend([format!("summon slime {gap} -26 0.5 0 0 minecraft:spawn_medium"), "effect @e[type=slime] slowness 60 255 true".into()]);
        waits(&mut lines, 4);
        lines.push("kill @e[type=slime]".into());
    }
    with(lines, &["kill @e[type=!player]", "gamemode creative @s"])
}

pub fn wind() -> Vec<String> {
    let mut lines = with(start("noon"), &["gamemode creative @s", "fill -8 -26 -7 8 -21 -6 stone"]);
    for _ in 0..2 {
        lines.extend(owned(["tp @s 0.5 -26 0.5 180 0", "summon cow -0.5 -26 -4.5", "summon cow 2.5 -26 -4.5", "summon cow -2.5 -26 -3.5", "summon cow 4.5 -26 -4.5"]));
        lines.extend(owned(["effect @e[type=cow] slowness 60 255 true", "replaceitem entity @s slot.weapon.mainhand 0 wind_charge 4", "tp @s 0.5 -26 2.5 180 0", "", "!use"]));
        waits(&mut lines, 4);
        lines.push("kill @e[type=cow]".into());
    }
    with(lines, &["fill -8 -26 -7 8 -21 -6 air", "kill @e[type=!player]"])
}

/// A 4x3x4 template of mixed blocks loaded at integrity 50 with no seed three times, then with seed "".
pub fn integrity() -> Vec<String> {
    let mut lines = with(start("noon"), &["gamemode creative @s", "fill 3 -26 3 6 -24 6 oak_planks", "fill 4 -25 4 5 -24 5 glass", "structure save q3s 3 -26 3 6 -24 6"]);
    for (x, seed) in [(-12, ""), (-6, ""), (8, ""), (14, " \"\"")] {
        lines.extend([format!("fill {x} -26 -12 {} -24 -9 air", x + 3), format!("structure load q3s {x} -26 -12 0_degrees none true true false 50{seed}"), String::new()]);
    }
    with(lines, &["fill -20 -26 -12 20 -24 6 air"])
}

/// The template loaded at integrity 50 with no seed by both players in one tick (`execute as @a`), twice.
pub fn integrity_twice() -> Vec<String> {
    let mut lines = with(start("noon"), &["gamemode creative @s", "fill 3 -26 3 6 -24 6 oak_planks", "fill 4 -25 4 5 -24 5 glass", "structure save q3s 3 -26 3 6 -24 6"]);
    for _ in 0..2 {
        lines.extend(owned(["fill -2 -26 -14 1 -24 -11 air", "fill 12 -26 0 15 -24 3 air", "execute as @a at @s run structure load q3s ~-2.5 -26 ~-14.5 0_degrees none true true false 50", ""]));
    }
    with(lines, &["fill -2 -26 -14 1 -24 -11 air", "fill 12 -26 0 15 -24 3 air", "fill 3 -26 3 6 -24 6 air"])
}

/// Suspicious gravel 2 east of the actor's feet, brushed 30 ticks, let go 1 s, 30 more, let go 2 s,
/// then to the end; the same with 10-tick strokes on sand 2 west.
pub fn brush() -> Vec<String> {
    let mut lines = with(start("noon"), &["gamemode survival @s", "setblock 2 -28 0 stone", "setblock -2 -28 0 stone", "setblock 2 -27 0 suspicious_gravel", "setblock -2 -27 0 suspicious_sand", "give @s brush", ""]);
    lines.extend(owned(["!brush 2 -1 0 30", "", "!brush 2 -1 0 30", "", "", "!brush 2 -1 0 200", "!brush -2 -1 0 10", "!brush -2 -1 0 10", "", "!brush -2 -1 0 200"]));
    with(lines, &["gamemode creative @s", "clear @s", "kill @e[type=item]", "fill -2 -27 0 2 -27 0 stone", "fill -2 -28 0 2 -28 0 air"])
}

/// Each kind put in a boat at (4.5, -26, 4.5) in turn: where its rider position sits over the boat.
pub fn seats() -> Vec<String> {
    let mut lines = with(start("noon"), &["gamemode creative @s", "difficulty peaceful"]);
    for kind in ["skeleton", "villager_v2", "pillager", "witch", "piglin", "pig", "sheep", "chicken", "wolf", "spider", "creeper", "fox", "panda", "goat", "frog"] {
        lines.extend(["summon boat 4.5 -26 4.5".into(), format!("summon {kind} 6.5 -26 4.5"), format!("ride @e[type={kind}] start_riding @e[type=boat] teleport_rider"), String::new(), "kill @e[type=!player]".into()]);
    }
    with(lines, &["difficulty easy"])
}

/// Twelve husks 40 off and twelve 10 off the creative actor, counted each 5 s for a minute.
pub fn despawn() -> Vec<String> {
    let mut lines = with(start("noon"), &["gamemode creative @s", "fill 36 -27 -8 44 -27 8 stone", "fill -20 -26 -20 20 -16 20 air"]);
    for z in -6..6 {
        lines.extend([format!("summon husk 40.5 -26 {z}.5"), format!("summon husk 10.5 -26 {z}.5")]);
    }
    for _ in 0..12 {
        lines.extend(owned(["testfor @e[type=husk,x=40,y=-26,z=0,r=15]", "testfor @e[type=husk,x=10,y=-26,z=0,r=15]", "", "", ""]));
    }
    with(lines, &["kill @e[type=!player]", "fill 36 -27 -8 44 -27 8 air"])
}

/// foes_drill.rs's `stare` at the head 6 off, twice with a carved pumpkin worn, then twice without.
pub fn stare() -> Vec<String> {
    let mut lines = with(start("midnight"), &["effect @s resistance 900 4 true"]);
    for worn in ["carved_pumpkin", "carved_pumpkin", "air", "air"] {
        lines.extend([format!("replaceitem entity @s slot.armor.head 0 {worn}"), "summon enderman ~ ~ ~6".into(), String::new(), "tp @e[type=enderman] ~ ~ ~6".into(), "effect @e[type=enderman] slowness 30 255".into()]);
        lines.extend(owned(["!look 0 2.6 6", "", "", "", "!look 0 1.62 -10", "kill @e[type=enderman]", ""]));
    }
    with(lines, &["kill @e[type=!player]", "gamemode creative @s"])
}

pub fn love() -> Vec<String> {
    let mut lines = with(start("noon"), &["tp @s 0.5 -26 0.5 180 0", "replaceitem entity @s slot.weapon.mainhand 0 wheat 64", "summon cow 0.5 -26 -1.5", "effect @e[type=cow] slowness 120 255 true", "", "!interact cow"]);
    waits(&mut lines, 34);
    lines.push("kill @e[type=cow]".into());
    for x in [-12, -4, 4, 12] {
        lines.extend([format!("tp @s {x}.5 -26 -4.5 180 0"), format!("summon cow {x}.5 -26 -6.5"), String::new(), "!interact cow".into(), format!("summon cow {}.5 -26 -6.5", x + 1), String::new(), "!interact cow".into()]);
    }
    lines.push("tp @s 0.5 -26 12.5 180 0".into());
    waits(&mut lines, 20);
    with(lines, &["kill @e[type=!player]", "gamemode creative @s"])
}
