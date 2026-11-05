//! The thirteenth wave's group S drills (research/parity/THIRTEENTH-BRIEF.md), on warden_drill.rs's floor
//! (stone at y -27, the warded survival actor at the middle) unless named. `s3witch`: a free witch on
//! magma (drinking), then a held one throwing at the actor stepping 2 aside each second (near
//! breaks). `s3drowned`: three trident drowned in a block of still water 6 deep, 6 to 10 off the
//! actor on the floor beside it. `s3evrange`: a held evoker's prey taken 9 off, then 21 to 69 off. `s3misc`: summoned boats'
//! spots, `tp` success counts, falling suspicious blocks landing. `s3brush`: q3brush's strokes with
//! a use a real tick apart (`!tbrush`). On a default world of seed 20261003 (sapling
//! `SAPLING_DRILL_WORLD=generated SAPLING_DRILL_SEED=20261003`): `s3melt`, snow golems in the desert
//! by (-6114, -3508); `s3despawn`, natural mobs by the swamp hut, the players then 44 off.

use std::time::Duration;

use super::creatures::with;
use super::warden::{healed, start};

#[path = "thirteenth3s_world.rs"]
mod world;

const SHOWN: &[&str] = &[
    "AddEntity", "RemoveEntity", "AddItemEntity", "TakeItemEntity", "MoveEntityDelta", "SetEntityMotion", "SetEntityData", "EntityEvent", "LevelSoundEvent", "LevelEvent", "MobEffect", "MobEquipment", "UpdateAttributes", "CommandOutput", "Animate", "SetEntityLink", "MoveEntityAbsolute",
];

const BLOCKS: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "BlockEntityData", "LevelSoundEvent", "LevelEvent", "AddItemEntity", "AddEntity", "RemoveEntity", "CommandOutput"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "s3witch" => (witch(), SHOWN),
        "s3drowned" => (drowned(), SHOWN),
        "s3witch2" => (witch_roofed(), SHOWN),
        "s3evrange" => (evoker_range(), SHOWN),
        "s3misc" => (misc(), BLOCKS),
        "s3brush" => (brush(), BLOCKS),
        "s3stare" => (stare(1), SHOWN),
        "s3stare2" => (stare(2), SHOWN),
        "s3stare3" => (stare(3), SHOWN),
        "s3far" => (far(), SHOWN),
        "s3wither" => (wither(), SHOWN),
        "s3poi" => (poi([7, 9, 11, 13]), &["CommandOutput", "AddEntity", "SetEntityData"]),
        "s3poi2" => (poi([16, 20, 26, 32]), &["CommandOutput", "AddEntity", "SetEntityData"]),
        "s3poi3" => (poi([14, 15, 14, 15]), &["CommandOutput", "AddEntity", "SetEntityData"]),
        "s3books" =>(books(), &["UpdateTrade", "CommandOutput", "AddEntity"]),
        "s3melt" => (world::melt(), SHOWN),
        "s3despawn" => (world::despawn(), &["CommandOutput", "AddEntity", "RemoveEntity"]),
        _ => return None,
    })
}

fn warded(time: &str, more: &[&str]) -> Vec<String> {
    with(start(time), &[&["effect @s resistance 900 4 true"], more].concat())
}

fn witch() -> Vec<String> {
    let mut lines = warded("midnight", &["fill -3 -27 -11 3 -27 -6 magma", "tp @s 0.5 -26 0.5 180 0", "summon witch 0.5 -26 -8.5"]);
    healed(&mut lines, 25);
    lines.extend(["kill @e[type=witch]", "fill -3 -27 -11 3 -27 -6 stone", "summon witch 0.5 -26 -5.5", "effect @e[type=witch] slowness 120 255 true"].map(String::from));
    for _ in 0..12 {
        lines.extend(["tp @s 2.5 -26 0.5 180 0", "tp @s 0.5 -26 0.5 180 0", "effect @s instant_health 1 4"].map(String::from));
    }
    with(lines, &["kill @e[type=!player]", "effect @s clear", "gamemode creative @s"])
}

/// A held witch 6 off throwing at the actor under a 3x3 glass roof at y -24 (its potions break on
/// the roof or beside the actor), the actor's effects cleared each 3 s so that each throw is poison.
fn witch_roofed() -> Vec<String> {
    let mut lines = warded("midnight", &["fill -1 -24 -1 1 -24 1 glass", "tp @s 0.5 -26 0.5 180 0", "summon witch 0.5 -26 -5.5", "effect @e[type=witch] slowness 120 255 true"]);
    for _ in 0..16 {
        lines.extend(["effect @s clear", "effect @s resistance 900 4 true", ""].map(String::from));
    }
    with(lines, &["kill @e[type=!player]", "fill -1 -24 -1 1 -24 1 air", "effect @s clear", "gamemode creative @s"])
}

fn drowned() -> Vec<String> {
    let mut lines = warded("midnight", &["fill 3 -26 -8 14 -21 8 water", "tp @s 0.5 -26 0.5 -90 0"]);
    lines.extend(["summon drowned 6.5 -25 -4.5 0 0 minecraft:as_ranged_adult", "summon drowned 8.5 -25 0.5 0 0 minecraft:as_ranged_adult", "summon drowned 10.5 -25 4.5 0 0 minecraft:as_ranged_adult"].map(String::from));
    healed(&mut lines, 40);
    with(lines, &["kill @e[type=!player]", "fill -20 -26 -20 20 -21 20 air", "gamemode creative @s"])
}

fn evoker_range() -> Vec<String> {
    let mut lines = warded("midnight", &["difficulty normal", "fill 0 -27 21 0 -27 70 stone", "tp @s 0.5 -26 0.5 180 0", "summon evocation_illager 0.5 -26 -8.5", "effect @e[type=evocation_illager] slowness 120 255 true"]);
    healed(&mut lines, 6);
    for z in [12, 20, 30, 45, 60] {
        lines.push(format!("tp @s 0.5 -26 {z}.5 180 0"));
        for _ in 0..5 {
            lines.extend(["kill @e[type=vex]".into(), "effect @s instant_health 1 4".into()]);
        }
    }
    with(lines, &["tp @s 0.5 -26 0.5", "kill @e[type=!player]", "fill 0 -27 21 0 -27 70 air", "gamemode creative @s", "difficulty easy"])
}

fn misc() -> Vec<String> {
    let mut lines = with(start("noon"), &["gamemode creative @s"]);
    for summon in ["boat 4.5 -26 4.5", "boat 4.5 -26 4.5 90 0", "boat 4.5 -26 4.5 180 0", "boat 4.5 -26 4.5 45 0", "boat -3.5 -26 -6.5", "boat 2.0 -26 2.0", "boat 2.3 -26 -7.8", "chest_boat 4.5 -26 4.5"] {
        lines.extend([format!("summon {summon}"), String::new(), "kill @e[type=!player]".into()]);
    }
    lines.extend(["summon cow -5.5 -26 -5.5", "summon cow -6.5 -26 -5.5", "summon cow -7.5 -26 -5.5", "effect @e[type=cow] slowness 60 255 true"].map(String::from));
    lines.extend(["tp @e[type=cow] @s", "tp @e[type=cow] -6.5 -26 -5.5", "tp @s @e[type=cow,c=1]", "tp @a @s", "tp @a 0.5 -26 0.5", "tp CmdWatcher 14.5 -26 14.5", "tp @e[type=cow,c=2] @s", "kill @e[type=cow]"].map(String::from));
    lines.extend(["setblock 6 -22 6 suspicious_sand [\"hanging\"=false]", "setblock 8 -22 6 suspicious_gravel [\"hanging\"=false]", "setblock 10 -22 6 suspicious_sand", "", "setblock 10 -22 7 stone", "", "", ""].map(String::from));
    with(lines, &["fill 6 -27 6 10 -21 7 air", "fill 6 -27 6 10 -27 7 stone", "kill @e[type=!player]"])
}

/// Three cows left while both players are 3000 off for 40 s (their chunks unloaded), then come back
/// to: what a mob loaded again is first sent.
fn far() -> Vec<String> {
    let mut lines = with(start("noon"), &["gamemode creative @a", "summon cow 3.5 -26 3.5", "summon cow 5.5 -26 3.5", "summon cow 7.5 -26 3.5", "effect @e[type=cow] slowness 120 255 true", "", "tp @a 3000.5 -60 0.5"]);
    super::creatures::waits(&mut lines, 40);
    lines.extend(["tp @a 0.5 -26 0.5", "", "", ""].map(String::from));
    with(lines, &["kill @e[type=!player]", "tp CmdWatcher 14.5 -26 14.5"])
}

/// An enderman held 6 off, then 12 off, looked at for 2 s with the look turned aside by each angle
/// (degrees, the offset beside its head in blocks); a new one each time.
fn stare(set: u8) -> Vec<String> {
    let mut lines = warded("midnight", &["tp @s 0.5 -26 0.5 0 0"]);
    let (near, far): (&[f32], &[f32]) = match set {
        1 => (&[4.0, 5.0, 6.0, 7.0, 8.0, 9.0], &[2.5, 3.5, 4.5, 5.5, 6.5]),
        2 => (&[0.0, 1.0, 1.6, 2.2, 2.8, 3.4, 0.0, 1.6, 2.8], &[0.0, 0.6, 1.2, 1.8, 0.6, 1.2]),
        // Where the Java game's `1 - 0.025 / d` and a fixed reach aside part (each angle three times: endermen teleport off).
        _ => (&[4.6, 5.0, 5.5, 4.6, 5.0, 5.5, 4.6, 5.0, 5.5], &[2.8, 3.2, 3.5, 2.8, 3.2, 3.5, 2.8, 3.2, 3.5]),
    };
    for (off, angles) in [(6, near), (12, far)] {
        for angle in angles {
            let aside = off as f32 * angle.to_radians().tan();
            lines.extend([format!("say {off} off at {angle}"), format!("summon enderman 0.5 -26 {off}.5"), String::new(), format!("tp @e[type=enderman] 0.5 -26 {off}.5"), "effect @e[type=enderman] slowness 30 255".into()]);
            lines.extend([format!("!look {aside} 2.5 {off}"), String::new(), String::new(), "!look 0 1.62 -10".into(), "kill @e[type=enderman]".into()]);
        }
    }
    with(lines, &["kill @e[type=!player]", "gamemode creative @s"])
}

/// twelfth8_drill.rs's lying Ts with their skulls along x (stem at z -2, then z 2), the actor's last skull
/// put from 2.5 off along z as the Ts along z were, where g8wither's went astray.
fn wither() -> Vec<String> {
    let skull = "wither_skeleton_skull [\"facing_direction\"=1]";
    let mut lines = with(start("noon"), &["gamemode creative @s", "replaceitem entity @s slot.weapon.mainhand 0 wither_skeleton_skull 64"]);
    for (stem, arms, stand, click) in [(-2, -1, "3.5 -26 2.5 180 45", "0 -1 -2"), (2, 1, "3.5 -26 -2.5 0 45", "0 -1 3")] {
        lines.extend([format!("tp @s {stand}"), String::new(), format!("tp @s {stand}")]);
        lines.extend([format!("setblock 4 -26 {stem} soul_sand"), format!("fill 3 -26 {arms} 5 -26 {arms} soul_sand"), format!("setblock 5 -26 0 {skull}"), format!("setblock 4 -26 0 {skull}"), format!("!place {click}"), String::new(), String::new()]);
        lines.extend(["kill @e[type=wither]", "fill 1 -26 -3 7 -24 3 air", ""].map(String::from));
    }
    with(lines, &["clear @s", ""])
}

/// Twelve librarians, each held and its offers opened (`UpdateTrade`): their books' enchantments and prices.
fn books() -> Vec<String> {
    let mut lines = with(start("noon"), &["gamemode creative @s"]);
    for _ in 0..12 {
        lines.extend(["summon villager 3.5 -26 0.5 90 0 minecraft:become_librarian", "effect @e[type=villager_v2] slowness 30 255 true", "tp @s 1.5 -26 0.5 -90 0", "", "!trade villager_v2 0 0", "", "kill @e[type=villager_v2]"].map(String::from));
    }
    with(lines, &["kill @e[type=!player]"])
}

/// tenth3f_drill.rs's `f3poi` with the villager free to walk and a bed by the bell (a lone bell
/// made no BDS armorer 7 off in 30 s): a blast furnace 7 to 13 off the bell.
fn poi(offsets: [i32; 4]) -> Vec<String> {
    let mut lines = super::villages::village(&[]);
    for (index, d) in offsets.into_iter().enumerate() {
        let x = 400 + 200 * index as i32;
        lines.extend([format!("tp @a {x}.5 -60 -6.5"), format!("fill {} -61 -40 {} -61 40 grass_block", x - 40, x + 40), String::new()]);
        lines.extend([format!("setblock {x} -60 0 bell"), format!("setblock {} -60 3 bed [\"direction\"=0]", x - 3), format!("setblock {} -60 0 blast_furnace", x + d), format!("summon villager {}.5 -60 1.5 0 0 minecraft:become_unskilled", x - 1)]);
        for _ in 0..12 {
            lines.extend([format!("testfor @e[type=villager_v2,family=armorer,x={x},y=-60,z=0,r=40]"), String::new(), String::new(), String::new(), String::new()]);
        }
    }
    with(lines, &["tp @a 0.5 -60 0.5", "kill @e[type=!player]", "gamemode creative @s", ""])
}

/// As thirteenth3q_more.rs's `brush`, each use held a real tick after the last.
fn brush() -> Vec<String> {
    let mut lines = with(start("noon"), &["gamemode survival @s", "setblock 2 -28 0 stone", "setblock -2 -28 0 stone", "setblock 2 -27 0 suspicious_gravel", "setblock -2 -27 0 suspicious_sand", "give @s brush", ""]);
    lines.extend(["!tbrush 2 -1 0 30", "", "!tbrush 2 -1 0 30", "", "", "!tbrush 2 -1 0 200", "!tbrush -2 -1 0 10", "!tbrush -2 -1 0 10", "", "!tbrush -2 -1 0 200"].map(String::from));
    with(lines, &["gamemode creative @s", "clear @s", "kill @e[type=item]", "fill -2 -27 0 2 -27 0 stone", "fill -2 -28 0 2 -28 0 air"])
}

/// Does `!tbrush x y z ticks`: block_ticks_drill.rs's `!brush`, each held use 50 ms after the last.
pub async fn act(actor: &mut acacia_bot::Bot, line: &str) -> bool {
    let Some(words) = line.strip_prefix("!tbrush ") else { return false };
    let numbers: Vec<i32> = words.split(' ').filter_map(|word| word.parse().ok()).collect();
    let (Some(feet), &[x, y, z, ticks]) = (actor.movement().and_then(|movement| movement.position()), &numbers[..]) else { return true };
    let pos = [feet[0].floor() as i32 + x, feet[1].floor() as i32 + y, feet[2].floor() as i32 + z];
    println!("## look: {:?}", actor.look_at([pos[0] as f32 + 0.5, pos[1] as f32 + 1.0, pos[2] as f32 + 0.5]).await);
    let face = acacia_bot::interact::Face::Up;
    super::brush_hold::press(actor, pos, face);
    for _ in 0..ticks {
        tokio::time::sleep(Duration::from_millis(50)).await;
        super::brush_hold::held(actor, pos, face);
    }
    super::brush_hold::release(actor, pos);
    println!("## let go");
    true
}
