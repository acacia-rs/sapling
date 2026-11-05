//! The thirteenth wave's group V drills (research/parity/THIRTEENTH-BRIEF.md), on creature_drill.rs's floor
//! (stone at y -27, the actor at 0.5 -26 0.5 facing east; clicks are offsets from its cell). `v2statue`: copper
//! golem statues set, put by hand and clicked. `v2ents`: shulker boxes put by hand; a shulker box with stacks, a
//! sign with text, a vault and a trial spawner that ran, each cloned (`v2ran`: the last two within reach). `v2tp`: teleports beside a sculk sensor.
//! `v2nether`: a bed's foot clicked in the Nether. `v2drip`: pointed dripstone falling on a survival player.

use super::creatures::{floor, waits};

const SHOWN: &[&str] = &[
    "UpdateBlock", "UpdateSubchunkBlocks", "BlockEntityData", "BlockEvent", "LevelEvent", "LevelEventGeneric", "LevelSoundEvent", "AddEntity", "AddItemEntity", "RemoveEntity",
    "UpdateAttributes", "EntityEvent", "SetHealth", "CommandOutput",
];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    let lines = match name {
        "v2statue" => statue(),
        "v2ents" => ents(),
        "v2ran" => ran(),
        "v2tp" => tp(),
        "v2nether" => nether(),
        "v2drip" => drip(),
        _ => return None,
    };
    Some((lines, SHOWN))
}

/// `!signtext x y z text`: writes `text` on the front of the sign at that offset from the actor's feet.
pub async fn act(actor: &mut acacia_bot::Bot, line: &str) -> bool {
    let Some(words) = line.strip_prefix("!signtext ") else { return false };
    let words: Vec<&str> = words.splitn(4, ' ').collect();
    let feet = actor.movement().and_then(|movement| movement.position()).map(|at| at.map(|c| c.floor() as i32));
    let (Some(feet), [x, y, z, text]) = (feet, &words[..]) else { return true };
    let [Ok(x), Ok(y), Ok(z)] = [x, y, z].map(|c| c.parse::<i32>()) else { return true };
    let at = [feet[0] + x, feet[1] + y, feet[2] + z];
    println!("## signed: {:?}", actor.write_sign(at, acacia_bot::signs::SignSide::Front, text).await);
    true
}

fn with(mut lines: Vec<String>, more: &[&str]) -> Vec<String> {
    lines.extend(more.iter().map(|line| line.to_string()));
    lines
}

/// A statue set and clicked with an empty hand; one put by hand and clicked; a waxed oxidized one set.
fn statue() -> Vec<String> {
    let mut lines = with(floor(), &["setblock 2 -26 3 copper_golem_statue", "", "clear @s", "", "!click 2 0 3 west", ""]);
    lines = with(lines, &["replaceitem entity @s slot.weapon.mainhand 0 copper_golem_statue 1", "", "!place 2 -1 1", "", "", "clear @s", "", "!click 2 0 1 west", ""]);
    lines = with(lines, &["setblock 4 -26 3 waxed_oxidized_copper_golem_statue", "", "fill 2 -26 0 4 -26 4 air", ""]);
    lines
}

/// Two shulker boxes put by hand; then, each cloned a cell or two along: a shulker box given stacks, a sign
/// written by hand, a vault a survival player opened with a key, a trial spawner whose zombies were killed.
fn ents() -> Vec<String> {
    let mut lines = with(floor(), &["replaceitem entity @s slot.weapon.mainhand 0 undyed_shulker_box 64", "", "!place 2 -1 -2", "", "!place 2 -1 -4", ""]);
    lines = with(lines, &["setblock 5 -26 -2 undyed_shulker_box", "replaceitem block 5 -26 -2 slot.container 0 stone 5", "replaceitem block 5 -26 -2 slot.container 3 apple 2", ""]);
    lines = with(lines, &["clone 5 -26 -2 5 -26 -2 7 -26 -2", "", ""]);
    lines = with(lines, &["replaceitem entity @s slot.weapon.mainhand 0 oak_sign 1", "", "!place 2 -1 4", "", "!signtext 2 0 4 hello", "", "", "clone 2 -26 4 2 -26 4 4 -26 4", "", ""]);
    lines = with(lines, &["setblock 9 -26 -4 vault", "gamemode survival @s", "replaceitem entity @s slot.weapon.mainhand 0 trial_key 1", "", "!click 9 0 -4 west"]);
    waits(&mut lines, 4);
    lines = with(lines, &["gamemode creative @s", "kill @e[type=item]", "clone 9 -26 -4 9 -26 -4 11 -26 -4", "", ""]);
    lines = with(lines, &["setblock 9 -26 2 trial_spawner", "replaceitem entity @s slot.weapon.mainhand 0 zombie_spawn_egg 1", "", "!click 9 0 2 west", "", "gamemode survival @s"]);
    for _ in 0..24 {
        lines = with(lines, &["", "kill @e[type=zombie]"]);
    }
    waits(&mut lines, 2);
    with(lines, &["gamemode creative @s", "kill @e[type=item]", "clone 9 -26 2 9 -26 2 11 -26 2", "", "", "fill 2 -26 -6 12 -26 6 air", ""])
}

/// `v2ents`' vault and trial spawner within reach: a vault a survival player opened with a key, a trial spawner
/// given zombies whose wave was killed, each cloned three cells east.
fn ran() -> Vec<String> {
    let mut lines = with(floor(), &["setblock 3 -26 -3 vault", "gamemode survival @s", "replaceitem entity @s slot.weapon.mainhand 0 trial_key 1", "", "!click 3 0 -3 west"]);
    waits(&mut lines, 4);
    lines = with(lines, &["gamemode creative @s", "kill @e[type=item]", "clone 3 -26 -3 3 -26 -3 6 -26 -3", "", ""]);
    lines = with(lines, &["setblock 3 -26 3 trial_spawner", "replaceitem entity @s slot.weapon.mainhand 0 zombie_spawn_egg 1", "", "!click 3 0 3 west", "", "gamemode survival @s"]);
    for _ in 0..24 {
        lines = with(lines, &["", "kill @e[type=zombie]"]);
    }
    waits(&mut lines, 2);
    with(lines, &["gamemode creative @s", "kill @e[type=item]", "clone 3 -26 3 3 -26 3 6 -26 3", "", "", "fill 2 -26 -6 12 -26 6 air", ""])
}

/// A sensor at 6 -25 4 on stone; the actor `/tp`d to a point off its cell's middle and onto itself, an armour
/// stand summoned and `/tp`d, the actor eating chorus fruit.
fn tp() -> Vec<String> {
    let mut lines = with(floor(), &["setblock 6 -26 4 stone", "setblock 6 -25 4 sculk_sensor"]);
    waits(&mut lines, 2);
    lines = with(lines, &["tp @s 1.2 -26 3.8 -90 0"]);
    waits(&mut lines, 3);
    lines = with(lines, &["tp @s ~ ~ ~"]);
    waits(&mut lines, 3);
    lines = with(lines, &["summon armor_stand 2.5 -26 6.5"]);
    waits(&mut lines, 3);
    lines = with(lines, &["tp @e[type=armor_stand] 3.5 -26 5.5"]);
    waits(&mut lines, 3);
    lines = with(lines, &["gamemode survival @s", "replaceitem entity @s slot.weapon.mainhand 0 chorus_fruit 1", "", "!eat"]);
    waits(&mut lines, 4);
    with(lines, &["gamemode creative @s", "kill @e[type=armor_stand]", "fill 6 -26 4 6 -25 4 air", ""])
}

/// The actor in the Nether first, then its floor cleared and laid, a bed set and its foot clicked.
fn nether() -> Vec<String> {
    let mut lines = with(floor(), &["execute in nether run tp @s 0.5 100 0.5 -90 0", "", "", "", "", "", "", ""]);
    lines = with(lines, &["fill -6 99 -6 12 99 12 obsidian", "fill -6 100 -6 12 106 12 air", "tp @s 0.5 100 0.5 -90 0", "", "setblock 0 100 4 bed [\"direction\"=0]", "", "!click 0 0 3", ""]);
    waits(&mut lines, 1);
    with(lines, &["execute in overworld run tp @s 0.5 -26 0.5 -90 0", "kill @e[type=item]", ""])
}

/// Hanging tips let go 10 and 20 blocks over a survival actor.
fn drip() -> Vec<String> {
    let mut lines = floor();
    for top in [-15, -5] {
        lines.extend(["gamemode survival @s".to_owned(), format!("setblock 0 {top} 0 stone"), format!("setblock 0 {} 0 pointed_dripstone [\"hanging\"=true,\"dripstone_thickness\"=\"tip\"]", top - 1), String::new()]);
        lines.extend([format!("setblock 0 {top} 0 air")]);
        waits(&mut lines, 3);
        lines.extend(["gamemode creative @s".to_owned(), "kill @e[type=item]".to_owned(), "fill 0 -26 0 0 -5 0 air".to_owned(), "effect @s instant_health 1 5".to_owned(), String::new()]);
    }
    lines
}
