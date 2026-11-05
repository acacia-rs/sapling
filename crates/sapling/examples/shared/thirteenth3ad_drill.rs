//! The thirteenth wave's group AD drills (research/parity/THIRTEENTH-BRIEF.md), on warden_drill.rs's floor
//! (stone at y -27, the actor at the middle). `ad3drops`: zombie pigmen killed by the actor's
//! `/damage` (their swords dropped by chance) and by `/kill` (none). `ad3armor`: skeletons summoned by
//! night on hard, then on normal (armour pools and enchanted bows by the special multiplier).
//! `ad3reload`: cows shown again as the players come back (their order).

use super::creatures::{waits, with};
use super::warden::start;

const SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "AddItemEntity", "MobEquipment", "MobArmorEquipment", "CommandOutput"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "ad3drops" => (drops(), SHOWN),
        "ad3armor" => (armor(), SHOWN),
        "ad3reload" => (reload(), SHOWN),
        _ => return None,
    })
}

/// Five cows summoned a block apart in one chunk (x 3.5 to 11.5), the second killed, then both
/// players 3000 off for 40 s and back: the order they are shown again in (EnTT's swap on removal
/// moves the newest into the dead one's place).
fn reload() -> Vec<String> {
    let mut lines = with(start("noon"), &["gamemode creative @a"]);
    lines.extend((0..5).map(|at| format!("summon cow {}.5 -26 3.5 0 0 minecraft:entity_spawned cow{at}", 3 + 2 * at)));
    lines.extend(["effect @e[type=cow] slowness 120 255 true", "", "kill @e[type=cow,name=cow1]", "", "tp @a 3000.5 -60 0.5"].map(String::from));
    waits(&mut lines, 40);
    lines.extend(["tp @a 0.5 -26 0.5", "", "", ""].map(String::from));
    with(lines, &["kill @e[type=!player]", "tp CmdWatcher 14.5 -26 14.5"])
}

/// `count` of `kind` summoned in rows of 8 north of the actor, held.
fn rows(lines: &mut Vec<String>, kind: &str, count: usize) {
    for at in 0..count as i32 {
        lines.push(format!("summon {kind} {}.5 -26 {}.5", at % 8 * 2 - 7, -4 - 2 * (at / 8)));
    }
    lines.push(format!("effect @e[type={kind}] slowness 120 255 true"));
}

/// Three rounds of 16 zombie pigmen the creative actor's `/damage` kills, then 16 `/kill`ed.
fn drops() -> Vec<String> {
    let mut lines = with(start("noon"), &["gamemode creative @s", "difficulty normal"]);
    for _ in 0..3 {
        rows(&mut lines, "zombie_pigman", 16);
        waits(&mut lines, 1);
        lines.extend(["damage @e[type=zombie_pigman] 100 entity_attack entity @s".to_owned(), String::new(), String::new(), "kill @e[type=item]".to_owned()]);
    }
    rows(&mut lines, "zombie_pigman", 16);
    waits(&mut lines, 1);
    with(lines, &["kill @e[type=zombie_pigman]", "", "", "kill @e[type=item]", "difficulty easy"])
}

/// 32 skeletons summoned by midnight on hard, then 32 on normal, the actor creative.
fn armor() -> Vec<String> {
    let mut lines = with(start("midnight"), &["gamemode creative @s"]);
    for difficulty in ["hard", "normal"] {
        lines.push(format!("difficulty {difficulty}"));
        rows(&mut lines, "skeleton", 32);
        waits(&mut lines, 2);
        lines.push("kill @e[type=skeleton]".to_owned());
        waits(&mut lines, 1);
    }
    with(lines, &["kill @e[type=item]", "difficulty easy", "time set noon"])
}
