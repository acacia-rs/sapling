//! The thirteenth wave's group Y drills (research/parity/THIRTEENTH-BRIEF.md), on warden_drill.rs's floor
//! (stone at y -27, the actor at the middle) unless named. `y3wither`: a wither on normal with both
//! players creative (its side heads' idle looks), then the warded survival actor put back each second
//! (where its skulls are aimed). `y3hunt`: a zombie behind a wall it had seen the actor across, one
//! with a hole in its way, one 28 off, a spider taking the actor by night as day comes, a held zombie
//! turning its head. `y3gear`: armed kinds summoned, what they are shown holding. `y3arrow`: a cow
//! shot and one hit (their throws), a lingering potion's cloud shrinking. `y3dragon`: a held ender
//! dragon in a lingering cloud. On the flat ground: `y3trial`, trial spawners' detection (eyes, walls);
//! `y3omen`, ominous ones' item spawners. On village_drill.rs's
//! flat ground, each left for the saved `VILLAGE_*_PLAYERS` to be read: `y3found` (none acting),
//! `y3baby` (a baby villager hit ten times by a village golem), `y3ranged` (a villager shot dead by
//! the actor by a village golem), `y3cure` (a villager hit once, then a zombie villager cured).

use super::creatures::{waits, with};
use super::warden::start;
use crate::villages::{UNSKILLED, VILLAGE_SHOWN, every, village};

const SHOWN: &[&str] = &[
    "AddEntity", "RemoveEntity", "MoveEntityDelta", "SetEntityMotion", "SetEntityData", "EntityEvent", "LevelSoundEvent", "LevelEvent", "MobEquipment", "MobArmorEquipment", "CommandOutput", "MoveEntityAbsolute", "MovePlayer",
];

const TRIAL_SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "AddItemEntity", "UpdateBlock", "BlockEntityData", "LevelEvent", "LevelSoundEvent", "SetEntityData", "CommandOutput", "MovePlayer"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "y3wither" => (wither(), SHOWN),
        "y3hunt" => (hunt(), SHOWN),
        "y3gear" => (gear(), SHOWN),
        "y3arrow" => (arrow(), SHOWN),
        "y3dragon" => (dragon(), &["AddEntity", "RemoveEntity", "EntityEvent", "UpdateAttributes", "SetEntityData", "CommandOutput", "LevelEvent"]),
        "y3trial" => (trial(), TRIAL_SHOWN),
        "y3omen" => (omen(), TRIAL_SHOWN),
        "y3found" => (found(), VILLAGE_SHOWN),
        "y3baby" => (baby(), VILLAGE_SHOWN),
        "y3ranged" => (ranged(), VILLAGE_SHOWN),
        "y3cure" => (cure(), VILLAGE_SHOWN),
        _ => return None,
    })
}

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

/// The warded actor put back on its spot each second for `seconds`, healed every other.
fn held(lines: &mut Vec<String>, seconds: usize) {
    for _ in 0..seconds / 2 {
        lines.extend(owned(["tp @s 0.5 -26 0.5", "effect @s instant_health 1 4"]));
    }
}

fn wither() -> Vec<String> {
    let mut lines = with(start("noon"), &["difficulty normal", "gamerule mobgriefing false", "gamemode creative @s", "effect @s resistance 900 4 true", "summon wither 0.5 -26 -12.5"]);
    waits(&mut lines, 75);
    lines.extend(owned(["gamemode survival @s", "tp @s 0.5 -26 0.5"]));
    held(&mut lines, 50);
    with(lines, &["gamemode creative @s", "kill @e[type=!player]", "gamerule mobgriefing true", "difficulty easy"])
}

fn hunt() -> Vec<String> {
    let mut lines = with(start("midnight"), &["effect @s resistance 900 4 true", "tp @s 0.5 -26 0.5 180 0", "summon zombie 0.5 -26 -9.5"]);
    // Seen, then a wall two high across the floor between.
    waits(&mut lines, 3);
    lines.push("fill -20 -26 -4 20 -25 -4 stone".into());
    held(&mut lines, 12);
    lines.extend(owned(["kill @e[type=zombie]", "fill -20 -26 -4 20 -25 -4 air", "tp @s 0.5 -26 0.5 180 0"]));
    // A hole three across and two deep straight in its way.
    lines.extend(owned(["fill -1 -28 -6 1 -28 -4 stone", "fill -1 -27 -5 1 -27 -5 air", "summon zombie 0.5 -26 -11.5"]));
    held(&mut lines, 12);
    lines.extend(owned(["kill @e[type=zombie]", "fill -1 -27 -5 1 -27 -5 stone", "fill -1 -28 -6 1 -28 -4 air"]));
    // 28 off on a stone strip, past `within_radius` 25.
    lines.extend(owned(["fill 21 -27 -1 31 -27 1 stone", "tp @s 0.5 -26 0.5 90 0", "summon zombie 28.5 -26 0.5", "effect @e[type=zombie] slowness 120 255 true"]));
    held(&mut lines, 12);
    lines.extend(owned(["kill @e[type=zombie]", "fill 21 -27 -1 31 -27 1 air"]));
    // A spider takes the actor by night, then day comes.
    lines.extend(owned(["tp @s 0.5 -26 0.5 180 0", "summon spider 0.5 -26 -9.5"]));
    held(&mut lines, 4);
    lines.push("time set noon".into());
    held(&mut lines, 12);
    lines.extend(owned(["kill @e[type=spider]", "time set midnight"]));
    // A held zombie 6 off as the actor goes round it.
    lines.extend(owned(["summon zombie 0.5 -26 -5.5", "effect @e[type=zombie] slowness 120 255 true", ""]));
    for at in ["6.5 -26 -5.5", "0.5 -26 0.5", "-5.5 -26 -5.5", "0.5 -26 -11.5", "6.5 -26 -5.5"] {
        lines.extend([format!("tp @s {at}"), String::new()]);
    }
    with(lines, &["kill @e[type=!player]", "gamemode creative @s", "time set noon"])
}

/// Armed kinds summoned one by one 9 off the creative actor and held.
fn gear() -> Vec<String> {
    let mut lines = with(start("noon"), &["gamemode creative @s", "difficulty easy"]);
    for kind in ["skeleton", "stray", "bogged", "wither_skeleton", "zombie_pigman", "piglin", "piglin_brute", "pillager", "vindicator", "drowned", "zombie", "husk", "vex", "evocation_illager"] {
        lines.extend([format!("summon {kind} 0.5 -26 -8.5"), format!("effect @e[type={kind}] slowness 120 255 true"), String::new(), format!("kill @e[type={kind}]"), String::new()]);
    }
    lines
}

/// A held cow shot from 6 by a full bow, then a held cow hit; a lingering potion of harming thrown 4
/// ahead and its cloud watched 32 s.
fn arrow() -> Vec<String> {
    let mut lines = with(start("noon"), &["gamemode creative @s", "replaceitem entity @s slot.weapon.mainhand 0 bow", "replaceitem entity @s slot.inventory 0 arrow 8", "gamemode survival @s", "tp @s 0.5 -26 0.5 180 0"]);
    lines.extend(owned(["summon cow 0.5 -26 -5.5", "effect @e[type=cow] slowness 120 255 true", "!look 0 0.9 -6", "!use", "", "!release", "", "", "kill @e[type=cow]", "kill @e[type=arrow]"]));
    lines.extend(owned(["summon cow 0.5 -26 -1.5", "effect @e[type=cow] slowness 120 255 true", "", "!hit cow", "", "", "kill @e[type=cow]", "kill @e[type=item]"]));
    lines.extend(owned(["replaceitem entity @s slot.weapon.mainhand 0 lingering_potion 1 23", "summon cow 0.5 -26 -4.5", "effect @e[type=cow] slowness 120 255 true", "!look 0 -1 -4", "!use", "!release"]));
    waits(&mut lines, 32);
    with(lines, &["kill @e[type=!player]", "gamemode creative @s", "clear @s"])
}

/// A lingering potion of harming thrown by a held cow, then an ender dragon summoned and put on the
/// cloud's spot each tick by a repeating command block; watched 20 s.
fn dragon() -> Vec<String> {
    let mut lines = with(start("noon"), &["gamemode creative @s", "gamerule mobgriefing false", "tp @s 0.5 -26 3.5 180 0", "setblock 3 -26 6 repeating_command_block"]);
    lines.extend(owned(["!cmdblock 3 0 3 1 0 0 0 0 0 execute at @e[type=area_effect_cloud] run tp @e[type=ender_dragon] ~ ~ ~", "summon cow 0.5 -26 -1.5", "effect @e[type=cow] slowness 120 255 true", "", "replaceitem entity @s slot.weapon.mainhand 0 lingering_potion 1 23", "!look 0 -1.6 -4", "!use", "!release", "", "summon ender_dragon 0.5 -26 -12.5"]));
    waits(&mut lines, 20);
    with(lines, &["setblock 3 -26 6 air", "kill @e[type=!player]", "gamerule mobgriefing true", "clear @s"])
}

/// Trial spawners stop with `doMobSpawning` off, on BDS as in sapling (spawners/trial.rs).
const TRIAL_START: [&str; 6] = ["kill @e[type=!player]", "gamerule domobspawning true", "difficulty easy", "time set noon", "effect @s resistance 600 255 true", "clear @s"];
const TRIAL_END: [&str; 5] = ["gamemode creative @s", "kill @e[type=!player]", "gamerule domobspawning false", "effect @s clear", "tp @s 0.5 -26 0.5"];

/// A trial spawner with a zombie at `[x, y, z]`, its egg used from a barrier 4 west.
fn spawner(lines: &mut Vec<String>, [x, y, z]: [i32; 3], ominous: bool) {
    let state = if ominous { " [\"ominous\"=true]" } else { "" };
    lines.extend([
        "gamemode creative @s".to_owned(), format!("setblock {} {} {z} barrier", x - 4, y - 1), format!("tp @s {}.5 {y} {z}.5", x - 4), String::new(), format!("fill {} {y} {} {} {} {} air", x - 3, z - 1, x - 1, y + 2, z + 1),
        format!("setblock {x} {y} {z} trial_spawner{state}"), "replaceitem entity @s slot.weapon.mainhand 0 zombie_spawn_egg".to_owned(), String::new(), "!click 4 0 0 west".to_owned(), String::new(), "clear @s".to_owned(),
    ]);
}

/// [`spawner`] at (`x`, `y`, 0) with `more` set round it, then the actor survival at `stand` for 8 s.
fn trial_site(lines: &mut Vec<String>, x: i32, y: i32, more: &[&str], stand: &str) {
    spawner(lines, [x, y, 0], false);
    lines.extend(more.iter().map(|line| line.to_string()));
    lines.extend([format!("tp @s {stand}"), "gamemode survival @s".to_owned()]);
    waits(lines, 8);
    lines.extend(["gamemode creative @s".to_owned(), "kill @e[type=zombie]".to_owned()]);
}

/// Trial spawners each tested alone, 40 apart on the flat ground: one 15 over the actor's feet (its
/// eyes 13.9 under it, range 14), one 8 off behind a stone wall, one behind glass, one in the open.
fn trial() -> Vec<String> {
    let mut lines = owned(TRIAL_START);
    trial_site(&mut lines, 60, -45, &[], "60.5 -60 0.5");
    trial_site(&mut lines, 100, -60, &["fill 99 -60 4 101 -57 4 stone"], "100.5 -60 8.5");
    trial_site(&mut lines, 140, -60, &["fill 139 -60 4 141 -57 4 glass"], "140.5 -60 8.5");
    trial_site(&mut lines, 180, -60, &[], "180.5 -60 8.5");
    with(lines, &TRIAL_END)
}

/// Eight ominous trial spawners in two rows 6 apart, the actor among them for 60 s: what each item
/// spawner carries (its `MinecartDisplayBlock`) and what it throws.
fn omen() -> Vec<String> {
    let mut lines = owned(TRIAL_START);
    for site in 0..8 {
        spawner(&mut lines, [220 + 6 * (site % 4), -60, 12 * (site / 4)], true);
    }
    lines.push("tp @s 229.5 -60 6.5".into());
    lines.push("gamemode survival @s".into());
    waits(&mut lines, 60);
    with(lines, &TRIAL_END)
}

const GOLEM: &str = "summon iron_golem 10.5 -60 -6.5 0 0 minecraft:from_village";

/// A held grown villager summoned into the village and hit once by the actor.
const HIT_ONE: [&str; 5] = [
    "summon villager 2.5 -60 -3.5 0 0 minecraft:become_unskilled", "effect @e[type=villager_v2] slowness 1000 255 true", "tag @e[type=villager_v2,x=2.5,y=-60,z=-3.5,r=1] add hit",
    "execute at @e[tag=hit] run tp @s ~ ~ ~-1.5 facing ~ ~1 ~", "!hit villager_v2",
];

/// [`HIT_ONE`], 10 s on a new villager summoned, watched 20 s: its founding reward on the standing.
fn found() -> Vec<String> {
    let mut out = village(&UNSKILLED);
    out.extend(owned(HIT_ONE));
    waits(&mut out, 10);
    out.extend(owned(["summon villager 8.5 -60 4.5 0 0 minecraft:become_unskilled", "effect @e[type=villager_v2] slowness 1000 255 true"]));
    every(&mut out, "testfor @e[type=villager_v2]", 10, 2);
    out
}

/// A baby villager hit ten times beside a village golem held still.
fn baby() -> Vec<String> {
    let mut out = village(&UNSKILLED);
    out.extend(owned([GOLEM, "effect @e[type=iron_golem] slowness 1000 255 true", "summon villager 2.5 -60 3.5 0 0 minecraft:entity_born", "effect @e[type=villager_v2] slowness 1000 255 true"]));
    waits(&mut out, 8);
    out.push("tag @e[type=villager_v2,x=2.5,y=-60,z=3.5,r=1] add hit".into());
    for _ in 0..10 {
        out.extend(owned(["execute at @e[tag=hit] run tp @s ~ ~ ~-1.5 facing ~ ~0.5 ~", "!hit villager_v2", "effect @e[tag=hit] instant_health 1 4"]));
    }
    waits(&mut out, 5);
    out
}

/// A held grown villager shot dead from 8 beside a village golem held still.
fn ranged() -> Vec<String> {
    let mut out = village(&UNSKILLED);
    out.extend(owned([
        GOLEM, "effect @e[type=iron_golem] slowness 1000 255 true", "summon villager 2.5 -60 -3.5 0 0 minecraft:become_unskilled", "effect @e[type=villager_v2] slowness 1000 255 true",
        "replaceitem entity @s slot.weapon.mainhand 0 bow", "replaceitem entity @s slot.inventory 0 arrow 16", "tp @s 2.5 -60 -11.5",
    ]));
    waits(&mut out, 6);
    for _ in 0..4 {
        out.extend(owned(["!look 0 1.2 8", "!use", "", "!release", "", "testfor @e[type=villager_v2,x=2.5,y=-60,z=-3.5,r=2]"]));
    }
    waits(&mut out, 5);
    out
}

/// [`HIT_ONE`], 10 s on a summoned zombie villager cured, watched until 30 s after its cure: the
/// cure's +5, and whether the villager it becomes gives a founding reward too.
fn cure() -> Vec<String> {
    let mut out = village(&UNSKILLED);
    out.extend(owned(HIT_ONE));
    waits(&mut out, 10);
    out.extend(owned([
        "summon zombie_villager_v2 8.5 -60 4.5", "effect @e[type=zombie_villager_v2] slowness 1000 255 true", "effect @e[type=zombie_villager_v2] fire_resistance 1000 0 true",
        "effect @e[type=zombie_villager_v2] weakness 1000 0 true", "replaceitem entity @s slot.weapon.mainhand 0 golden_apple 1", "execute at @e[type=zombie_villager_v2] run tp @s ~ ~ ~-1.5", "",
        "!interact zombie_villager_v2", "",
    ]));
    every(&mut out, "testfor @e[type=zombie_villager_v2]", 10, 36);
    out
}
