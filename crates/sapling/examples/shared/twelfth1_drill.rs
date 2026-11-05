//! The twelfth wave's group 4 drills (research/parity/TWELFTH-BRIEF.md): calves of a Nether or
//! End cow sent into the overworld and an overworld cow (`w1inherit`); two players' loading screens
//! about a rejoin (`w1screens`); a book page's longest kept length (`w1pages`).

use super::creatures::{waits, with};

const SHOWN: &[&str] = &[
    "AddEntity", "RemoveEntity", "AddItemEntity", "SetEntityLink", "SetEntityData", "EntityEvent", "LevelSoundEvent", "LevelEvent", "MobEffect", "UpdateAttributes", "SetHealth",
    "MobEquipment", "MobArmorEquipment", "UpdateBlock", "BlockEntityData", "CommandOutput", "Text",
];

const SCREENS: &[&str] = &["ChangeDimension", "CommandOutput", "PlayStatus", "Respawn"];
const PAGES: &[&str] = &["CommandOutput", "Disconnect"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "w1inherit" => (inherited(), SHOWN),
        "w1screens" => (screens(), SCREENS),
        "w1pages" => (pages(), PAGES),
        _ => return None,
    })
}

/// The loading screens' numbers: the actor to the End and back, the watcher the same, then the
/// actor again after a rejoin (BDS went on counting past one in t2ecredits).
fn screens() -> Vec<String> {
    let trip = |who: &str| [format!("execute as {who} in the_end run tp @s 0.5 80 40.5"), String::new(), String::new(), String::new(), format!("execute as {who} in overworld run tp @s 0.5 -26 0.5"), String::new(), String::new(), String::new()];
    let mut lines = owned(["gamemode creative @a", "tp CmdWatcher 0.5 -26 3.5", ""]);
    lines.extend(trip("@s"));
    lines.extend(trip("CmdWatcher"));
    lines.extend(owned(["!rejoin", "", "", ""]));
    lines.extend(trip("@s"));
    lines.extend(trip("CmdWatcher"));
    lines
}

/// Pages of a rising length written raw into a book and read back, until BDS lets its writer go
/// (t2ebook1: at 1000; t12book kept 513).
fn pages() -> Vec<String> {
    let mut lines = owned(["clear @s", "replaceitem entity @s slot.weapon.mainhand 0 writable_book", "", "!write x", ""]);
    for length in [514, 600, 700, 768, 769, 800, 900, 999] {
        lines.extend([format!("!rawpage {}", "d".repeat(length)), String::new(), "!inv".to_owned()]);
    }
    with(lines, &["clear @s"])
}

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

/// Four glass pens on the flat ground: a cow summoned in the Nether (the first two) or the End
/// (the last two) sent into each by `/execute in overworld run tp`, an overworld cow beside it,
/// both fed; 25 s for the calves.
fn inherited() -> Vec<String> {
    const PENS: [i32; 4] = [-5, -2, 1, 4];
    let mut lines = owned(["kill @e[type=!player]", "gamerule domobspawning false", "time set noon", "weather clear", "gamemode creative @s", "fill -3 -60 -7 4 -57 7 air", ""]);
    for (dimension, pens) in [("nether", &PENS[..2]), ("the_end", &PENS[2..])] {
        // Sapling's debug build made the Nether's columns too slowly for a summon 2 s on.
        lines.extend([format!("execute in {dimension} run tp @s 0.5 80 40.5"), String::new(), String::new(), String::new(), String::new()]);
        for z in pens {
            lines.extend(["summon cow ~ ~ ~".to_owned(), format!("execute as @e[type=cow,c=1] in overworld run tp @s 1.5 -60 {}", *z as f32 + 0.5)]);
        }
    }
    lines.extend(owned(["execute in overworld run tp @s -4.5 -60 0.5", "", ""]));
    for z in PENS {
        lines.push(format!("summon cow -0.5 -60 {}", z as f32 + 0.5));
    }
    lines.extend(owned(["replaceitem entity @s slot.weapon.mainhand 0 wheat 64", ""]));
    // The bot feeds within 2.8 of its eyes and in sight (u1inherit and the first run here fed two
    // of eight through the glass), so each pair is fed before the pens close.
    for z in PENS {
        lines.extend([format!("tp @s 0.5 -60 {}", z as f32 + 0.5), String::new(), "!feedall cow".to_owned()]);
    }
    lines.extend(owned(["tp @s -4.5 -60 0.5", "fill -2 -60 -7 -2 -59 7 glass", "fill 3 -60 -7 3 -59 7 glass"]));
    lines.extend(PENS.iter().map(|z| format!("fill -1 -60 {} 2 -59 {} glass", z - 1, z - 1)));
    lines.push("fill -1 -60 6 2 -59 6 glass".to_owned());
    waits(&mut lines, 25);
    with(lines, &["testfor @e[type=cow]", "kill @e[type=!player]", "fill -3 -60 -7 4 -57 7 air", ""])
}
