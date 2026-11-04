//! The eleventh wave's re-checks of lines left undrillable (research/parity/ELEVENTH-BRIEF.md "Group 3"):
//! skeleton horse traps set by `minecraft:set_trap` (`u1trap`); a flaming arrow at TNT, with
//! `mobGriefing` on and off (`u1fire`), and a blaze's fireballs the same (`u1blaze`); bone meal dispensed on sea grass and kelp (`u1meal`); frosted
//! ice by day, by night and lit (`u1frost`, open sky: `SAPLING_DRILL_WORLD=open` on sapling); a burning
//! actor joining again (`u1rejoin`); a glide ended by flight and one into a wall (`u1fly`); an End gateway
//! watched (`u1gate`, seed 20261018 on BDS); a trial spawner under Trial Omen and one set ominous
//! (`u1omen`); a cold world's freezing and snow (`u1cold`, a flat ice plains world on both sides); a
//! feature and feature rules placed on grass (`u1place`); squids in a shut box of still water (`u1squid`).

use acacia_bot::Bot;

use super::creatures::{floor, waits, with};

const SHOWN: &[&str] = &[
    "AddEntity", "RemoveEntity", "AddItemEntity", "SetEntityLink", "SetEntityMotion", "SetEntityData", "EntityEvent", "LevelSoundEvent", "LevelEvent", "MobEffect", "UpdateAttributes",
    "SetHealth", "MobEquipment", "MobArmorEquipment", "UpdateBlock", "UpdateSubchunkBlocks", "BlockEvent", "BlockEntityData", "CommandOutput", "Text", "UpdateAbilities", "AddPlayer",
    "CorrectPlayerMovePrediction",
];

const SWIMS: &[&str] = &["AddEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityMotion", "SetEntityData", "CommandOutput"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "u1trap" => (traps(), SHOWN),
        "u1fire" => (flaming_arrows(), SHOWN),
        "u1meal" => (dispensed_meal(), SHOWN),
        "u1frost" => (frosted_ice(), SHOWN),
        "u1rejoin" => (burning_rejoin(), SHOWN),
        "u1fly" => (glides(), SHOWN),
        "u1gate" => (gateway(), SHOWN),
        "u1omen" => (omens(), SHOWN),
        "u1cold" => (cold(), SHOWN),
        "u1place" => (placed(), SHOWN),
        "u1blaze" => (blaze_griefing(), SHOWN),
        "u1squid" => (squids(), SWIMS),
        _ => return None,
    })
}

/// `!fly` and `!unfly` start and stop the actor's creative flight; false for any other line.
pub async fn act(actor: &mut Bot, line: &str) -> bool {
    let fly = match line {
        "!fly" => true,
        "!unfly" => false,
        _ => return false,
    };
    if let Some(controls) = actor.controls() {
        controls.fly = fly;
    }
    true
}

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

/// A lone trap 40 off, then a trap 14 off the creative actor that walks to 6 off it, then one the
/// survival actor comes to; the lone one is looked for 60 s after it was set.
fn traps() -> Vec<String> {
    let mut lines = with(floor(), &["time set noon", "gamerule dofiretick false", "setblock 40 -27 0 stone", "summon skeleton_horse 40.5 -26 0.5 0 0 minecraft:set_trap", "summon skeleton_horse 14.5 -26 0.5 0 0 minecraft:set_trap"]);
    waits(&mut lines, 3);
    lines.extend(owned(["testfor @e[type=skeleton]", "tp @s 8.5 -26 0.5"]));
    waits(&mut lines, 10);
    lines.extend(owned(["testfor @e[type=skeleton]", "testfor @e[type=skeleton_horse]", "kill @e[type=skeleton]", "kill @e[type=skeleton_horse,x=10,y=-26,z=0,r=20]", "kill @e[type=item]", "tp @s 0.5 -26 0.5", "fill -8 -26 -8 16 -16 8 air"]));
    lines.extend(owned(["summon skeleton_horse 14.5 -26 0.5 0 0 minecraft:set_trap", "gamemode survival @s", "effect @s health_boost 60 4 true", "effect @s instant_health 1 9 true", "", "tp @s 8.5 -26 0.5"]));
    waits(&mut lines, 10);
    lines.extend(owned(["testfor @e[type=skeleton]", "kill @e[type=skeleton]", "kill @e[type=skeleton_horse,x=10,y=-26,z=0,r=20]", "gamemode creative @s", "effect @s clear", "tp @s 0.5 -26 0.5"]));
    waits(&mut lines, 28);
    with(lines, &["testfor @e[type=skeleton_horse]", "kill @e[type=!player]", "setblock 40 -27 0 air", "fill -8 -27 -8 16 -27 8 stone", "fill -8 -26 -8 16 -16 8 air", "gamerule dofiretick true", ""])
}

/// One shot of the creative actor at TNT on a post 8 off: a plain arrow, then flaming ones with
/// `mobGriefing` on and off.
fn flaming_arrows() -> Vec<String> {
    let post = ["setblock 8 -26 0 stone", "setblock 8 -25 0 tnt", ""];
    let shot = ["tp @s 0.5 -26 0.5 -90 0", "", "!look 8 1.4 0", "!use", "", "!release", "", "", "testforblock 8 -25 0 tnt"];
    // The watcher stands aside: in the actor it took the shot and threw it back (bds-u1fire.txt).
    let mut lines = with(floor(), &["time set noon", "tp CmdWatcher 0.5 -26 5.5", "replaceitem entity @s slot.weapon.mainhand 0 bow"]);
    lines.extend(owned(post));
    lines.extend(owned(shot));
    lines.extend(owned(["kill @e[type=arrow]", "enchant @s flame 1", ""]));
    for griefing in ["true", "false"] {
        lines.push(format!("gamerule mobgriefing {griefing}"));
        lines.extend(owned(post));
        lines.extend(owned(shot));
        waits(&mut lines, 5);
        lines.extend(owned(["testforblock 8 -25 0 tnt", "kill @e[type=!player]", "fill -8 -27 -8 16 -27 8 stone", "fill -8 -26 -8 16 -16 8 air"]));
    }
    with(lines, &["gamerule mobgriefing true", "clear @s", ""])
}

/// A held blaze 10 off the survival actor (fire resistant) shooting 15 s with `mobGriefing` on, then 15 s off.
fn blaze_griefing() -> Vec<String> {
    let mut lines = with(floor(), &["time set midnight", "gamerule dofiretick false", "difficulty normal", "gamemode survival @s", "effect @s fire_resistance 60 0 true", "effect @s health_boost 60 4 true", "effect @s instant_health 1 9 true"]);
    lines.extend(owned(["summon blaze 10.5 -26 0.5", "effect @e[type=blaze] slowness 60 255 true"]));
    waits(&mut lines, 15);
    lines.push("gamerule mobgriefing false".into());
    waits(&mut lines, 15);
    with(lines, &["kill @e[type=!player]", "gamerule mobgriefing true", "gamemode creative @s", "effect @s clear", "difficulty easy", "fill -8 -26 -8 16 -16 8 air", "gamerule dofiretick true", ""])
}

/// A glass pool on sand with sea grass and kelp, each fed twice by a dispenser of bone meal in the
/// wall a redstone block powers.
fn dispensed_meal() -> Vec<String> {
    let mut lines = with(floor(), &["fill 2 -28 -4 8 -28 4 stone", "fill 2 -27 -4 8 -22 4 glass", "fill 3 -27 -3 7 -27 3 sand", "fill 3 -26 -3 7 -23 3 water", "setblock 5 -26 -3 seagrass", "setblock 5 -26 3 kelp"]);
    lines.extend(owned(["setblock 5 -26 -4 dispenser [\"facing_direction\"=3]", "setblock 5 -26 4 dispenser [\"facing_direction\"=2]"]));
    lines.extend(owned(["replaceitem block 5 -26 -4 slot.container 0 bone_meal 16", "replaceitem block 5 -26 4 slot.container 0 bone_meal 16", ""]));
    for _ in 0..2 {
        lines.extend(owned(["setblock 5 -26 -5 redstone_block", "setblock 5 -26 5 redstone_block", "", "", "setblock 5 -26 -5 air", "setblock 5 -26 5 air", ""]));
    }
    lines.extend(owned(["testforblock 5 -26 -3 seagrass", "testforblock 5 -25 -3 seagrass", "testforblock 5 -25 3 kelp", "testforblock 5 -24 3 kelp"]));
    with(lines, &["fill 2 -28 -5 8 -22 5 air", "fill -8 -27 -8 16 -27 8 stone", "kill @e[type=!player]", ""])
}

/// 3x3 frosted ice in a 5x5 pond, watched 20 s at noon; then at midnight, beside one lit by glowstone.
fn frosted_ice() -> Vec<String> {
    let pond = |x: i32| [format!("fill {} -26 1 {} -26 7 glass", x - 1, x + 5), format!("fill {x} -26 2 {} -26 6 water", x + 4), format!("fill {} -26 3 {} -26 5 frosted_ice", x + 1, x + 3)];
    let mut lines = with(floor(), &["time set noon"]);
    lines.extend(pond(2));
    waits(&mut lines, 20);
    lines.extend(owned(["time set midnight", "fill 1 -26 1 7 -26 7 air", ""]));
    lines.extend(pond(2));
    lines.extend(pond(10));
    lines.extend(owned(["setblock 12 -24 4 glowstone", "setblock 14 -24 4 glowstone"]));
    waits(&mut lines, 20);
    with(lines, &["fill -8 -26 -8 16 -16 8 air", "time set noon", ""])
}

/// The survival actor (resistant, as a fire resistant one was never shown alight) set alight in
/// lava, stepped out and joined again while burning.
fn burning_rejoin() -> Vec<String> {
    let lines = with(floor(), &["fill 2 -26 -1 4 -26 1 glass", "setblock 3 -26 0 lava", "gamemode survival @s", "effect @s resistance 60 255 true", "tp @s 3.5 -26 0.5", "", "tp @s 0.5 -26 0.5", "!rejoin", "", "", ""]);
    with(lines, &["fill 2 -26 -1 4 -26 1 air", "gamemode creative @s", "effect @s clear", ""])
}

/// A creative glide high up that turns to flight; then a survival glide into a stone wall.
fn glides() -> Vec<String> {
    let mut lines = with(floor(), &["replaceitem entity @s slot.armor.chest 0 elytra", "tp @s 0.5 20 0.5 -90 0", "!glide", "", "", "!fly", "", "", "", "!unfly", "", "", "tp @s 0.5 -26 0.5", ""]);
    lines.extend(owned(["fill 34 -60 -10 34 20 10 stone", "gamemode survival @s", "effect @s health_boost 60 4 true", "effect @s instant_health 1 9 true", "tp @s 0.5 8 0.5 -90 25", "!glide", "!look 1 -0.45 0"]));
    waits(&mut lines, 6);
    with(lines, &["gamemode creative @s", "effect @s clear", "clear @s", "fill 34 -60 -10 34 20 10 air", "tp @s 0.5 -26 0.5", ""])
}

/// The actor 3 off the worldgen gateway at (1953, 66, 181) in the End for 150 s.
fn gateway() -> Vec<String> {
    let mut lines = owned(["kill @e[type=!player]", "gamerule domobspawning false", "execute in the_end run tp @s 1956.5 66 181.5"]);
    waits(&mut lines, 150);
    with(lines, &["execute in overworld run tp @s 0.5 -26 0.5", ""])
}

/// A trial spawner woken by the survival actor under Trial Omen, watched 30 s; then one set ominous.
fn omens() -> Vec<String> {
    let mut lines = super::redstone_parts::stage([0, 30], [0, 30]);
    lines.extend(owned(["gamerule domobspawning true", "difficulty easy", "effect @s resistance 300 255 true", "effect @s trial_omen 600 0", "setblock 14 -60 10 trial_spawner", "", "replaceitem entity @s slot.weapon.mainhand 0 zombie_spawn_egg"]));
    lines.extend(owned(["!click 4 0 0 west", "", "clear @s", "gamemode survival @s"]));
    waits(&mut lines, 30);
    lines.extend(owned(["kill @e[type=zombie]", "gamemode creative @s", "setblock 14 -60 10 air", "setblock 14 -60 16 trial_spawner [\"ominous\"=true]", "", "replaceitem entity @s slot.weapon.mainhand 0 zombie_spawn_egg", "tp @s 10.5 -60 16.5"]));
    lines.extend(owned(["!click 4 0 0 west", "", "clear @s", "gamemode survival @s"]));
    waits(&mut lines, 30);
    with(lines, &["gamemode creative @s", "clear @s", "effect @s clear", "gamerule domobspawning false", "kill @e[type=!player]", "fill 0 -60 0 30 -55 30 air", ""])
}

/// A tree feature and two feature rules placed on grass at the flat world's ground.
fn placed() -> Vec<String> {
    let mut lines = owned(["kill @e[type=!player]", "gamerule domobspawning false", "fill 20 -61 -8 60 -61 8 grass_block", "fill 20 -60 -8 60 -45 8 air", "tp @s 40.5 -60 -6.5", ""]);
    for place in ["place feature minecraft:oak_tree_feature 30 -60 0", "place featurerule minecraft:plains_surface_flowers_feature 40 -60 0", "place featurerule minecraft:overworld_surface_tall_grass_feature 50 -60 0"] {
        lines.extend([place.to_owned(), String::new(), String::new()]);
    }
    with(lines, &["fill 20 -60 -8 60 -45 8 air", "tp @s 0.5 -26 0.5", ""])
}

/// A squid and a glow squid shut in a glass box of still water, watched 30 s.
fn squids() -> Vec<String> {
    let mut lines = with(floor(), &["fill 2 -27 -4 10 -20 4 glass", "fill 3 -26 -3 9 -21 3 water", "summon squid 6.5 -24 -1.5", "summon glow_squid 6.5 -24 1.5"]);
    waits(&mut lines, 30);
    with(lines, &["kill @e[type=!player]", "fill 2 -27 -4 10 -20 4 air", "fill -8 -27 -8 16 -27 8 stone", ""])
}

/// On the ground of an ice plains world: a pond open to the sky and one by a torch, watched 40 s of
/// snow at `randomTickSpeed` 30, then 20 s clear.
fn cold() -> Vec<String> {
    let mut lines = owned(["kill @e[type=!player]", "gamerule domobspawning false", "weather clear", "time set noon", "gamemode creative @s", "fill 0 -60 0 20 -50 20 air", "tp @s 10.5 -60 10.5"]);
    lines.extend(owned(["fill 2 -61 2 6 -61 6 water", "fill 12 -61 2 16 -61 6 water", "setblock 14 -60 7 torch", "gamerule randomtickspeed 30", "weather rain"]));
    waits(&mut lines, 40);
    lines.extend(owned(["weather clear"]));
    waits(&mut lines, 20);
    with(lines, &["gamerule randomtickspeed 1", ""])
}
