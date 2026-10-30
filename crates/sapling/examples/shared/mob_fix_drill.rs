//! The follow-up drills of mobs (FIX-BRIEF group 2): who zombies and spiders take and let go,
//! health boost and armour on mobs, what a chested mount drops and keeps, and sniffers digging
//! and hatching (docs/research/mob-attacks.md, mob-damage.md, mob-life.md, leftovers.md).

use super::creatures::{floor, waits, with};
use super::eggs::penned;
use super::harm::{SHOWN, start};

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some((
        match name {
            "targets2" => targets(),
            "boosts" => boosts(),
            "chestdie" => chest_die(),
            "sniffdig" => sniff_dig(),
            _ => return None,
        },
        SHOWN,
    ))
}

fn healed(lines: &mut Vec<String>, seconds: usize) {
    lines.extend(std::iter::repeat_n("effect @s instant_health 1 4".to_owned(), seconds));
}

/// Every mob gone and the actor back on its mark, so knockback never carries it off the floor.
fn reset(lines: &mut Vec<String>) {
    lines.extend(["kill @e[type=!player]", "effect @s clear", "tp @s 0.5 -26 0.5", "effect @s instant_health 1 4", ""].map(String::from));
}

/// Four of `kind` held 5 off the actor on each side, out of their leap's reach.
fn ringed(lines: &mut Vec<String>, kind: &str) {
    lines.extend([[5.5, 0.5], [-4.5, 0.5], [0.5, 5.5], [0.5, -4.5]].map(|[x, z]| format!("summon {kind} {x} -26 {z}")));
    lines.push("effect @e[type=!player] slowness 120 255 true".into());
}

/// A survival actor at midnight: a zombie 3 off on hard, a baby zombie 10 off on easy; four cave
/// spiders, then four spiders, held 5 off for 20 s (when they take and drop it); a free cave
/// spider 3 off (its bite and poison on normal).
pub fn targets() -> Vec<String> {
    let mut lines = start("midnight", 0);
    lines.extend(["gamemode survival @s", ""].map(String::from));
    lines.extend(["difficulty hard", "summon zombie 0.5 -26 -2.5"].map(String::from));
    healed(&mut lines, 6);
    reset(&mut lines);
    lines.extend(["difficulty easy", "summon zombie 0.5 -26 -9.5 0 0 minecraft:as_baby"].map(String::from));
    healed(&mut lines, 6);
    reset(&mut lines);
    lines.push("difficulty normal".into());
    for kind in ["cave_spider", "spider"] {
        ringed(&mut lines, kind);
        healed(&mut lines, 20);
        reset(&mut lines);
    }
    lines.push("summon cave_spider 0.5 -26 -2.5".into());
    healed(&mut lines, 10);
    reset(&mut lines);
    lines.extend(["gamemode creative @s", "difficulty easy", ""].map(String::from));
    lines
}

/// Health boost on a held cow: given, healed, hurt, given again weaker, cleared, given briefly and
/// left to run out; on a zombie, given and cleared. A zombie in a diamond chestplate hit by a
/// diamond sword and by `/damage`.
pub fn boosts() -> Vec<String> {
    let mut lines = start("midnight", 0);
    lines.extend(["summon cow 2.5 -26 0.5", "summon zombie 0.5 -26 -2.5", "effect @e[type=!player] slowness 120 255 true", ""].map(String::from));
    lines.extend(
        [
            "effect @e[type=cow] health_boost 30 4 true",
            "",
            "effect @e[type=cow] instant_health 1 4 true",
            "",
            "damage @e[type=cow] 5",
            "",
            "effect @e[type=cow] health_boost 30 1 true",
            "",
            "effect @e[type=cow] clear",
            "",
            "effect @e[type=cow] health_boost 2 0 true",
            "",
            "",
            "",
            "effect @e[type=zombie] health_boost 30 0 true",
            "",
            "effect @e[type=zombie] clear",
            "",
            "tp @s 0.5 -26 0.5",
            "replaceitem entity @e[type=zombie] slot.armor.chest 0 diamond_chestplate",
            "replaceitem entity @s slot.weapon.mainhand 0 diamond_sword",
            "",
            "!hit zombie",
            "",
            "",
            "damage @e[type=zombie] 8",
            "",
            "",
        ]
        .map(String::from),
    );
    with(lines, &["kill @e[type=!player]", "clear @s", ""])
}

/// A tamed donkey given a chest by a plain click, then a saddle and goods by command, killed with
/// `doMobLoot` on; then a chested donkey, a carpeted and chested llama and a saddled, armoured horse
/// left in the world for its folder to be read (`actor_dump`).
pub fn chest_die() -> Vec<String> {
    let mut lines = with(floor(), &["time set noon", "gamerule domobloot true"]);
    let tamed = |lines: &mut Vec<String>, kind: &str, z: i32| {
        lines.extend([format!("summon {kind} 2.5 -26 {z}.5 0 0 minecraft:spawn_tame_adult"), format!("effect @e[type={kind}] slowness 200 255 true"), String::new()]);
    };
    let click = |lines: &mut Vec<String>, kind: &str, item: &str, z: i32| {
        lines.extend(["clear @s".to_owned(), format!("give @s {item}"), format!("tp @s 0.5 -26 {z}.5 -90 0"), String::new(), format!("!interact {kind}"), String::new()]);
    };
    let goods = |lines: &mut Vec<String>, kind: &str| {
        lines.extend([format!("replaceitem entity @e[type={kind}] slot.saddle 0 saddle"), format!("replaceitem entity @e[type={kind}] slot.inventory 0 diamond 5"), format!("replaceitem entity @e[type={kind}] slot.inventory 2 emerald 2"), String::new()]);
    };
    tamed(&mut lines, "donkey", 0);
    click(&mut lines, "donkey", "chest", 0);
    goods(&mut lines, "donkey");
    lines.extend(["kill @e[type=donkey]", "", "", "kill @e[type=item]", ""].map(String::from));
    tamed(&mut lines, "donkey", 0);
    click(&mut lines, "donkey", "chest", 0);
    goods(&mut lines, "donkey");
    tamed(&mut lines, "llama", 4);
    click(&mut lines, "llama", "white_carpet", 4);
    click(&mut lines, "llama", "chest", 4);
    lines.extend(["replaceitem entity @e[type=llama] slot.inventory 0 diamond 3".to_owned(), String::new()]);
    tamed(&mut lines, "horse", -4);
    lines.extend(["replaceitem entity @e[type=horse] slot.saddle 0 saddle", "replaceitem entity @e[type=horse] slot.armor.body 0 iron_horse_armor", ""].map(String::from));
    with(lines, &["clear @s", "tp @s 0.5 -26 0.5", ""])
}

/// Two sniffers on a dirt and moss field sent searching (`/event ... on_scenting_success`) twice a
/// minute apart, and three sniffer eggs about to hatch on moss in the open.
pub fn sniff_dig() -> Vec<String> {
    let mut lines = penned(&["time set noon", "fill -7 -27 -7 15 -27 7 dirt", "fill -7 -27 -7 3 -27 7 moss_block"]);
    for [x, z] in [[-4, -4], [0, 4], [2, -2]] {
        lines.push(format!("setblock {x} -26 {z} sniffer_egg [\"cracked_state\"=\"max_cracked\"]"));
    }
    lines.extend(["summon sniffer 10.5 -26 -3.5", "summon sniffer 10.5 -26 3.5", "", "event entity @e[type=sniffer] on_scenting_success"].map(String::from));
    waits(&mut lines, 60);
    lines.push("event entity @e[type=sniffer] on_scenting_success".into());
    waits(&mut lines, 170);
    lines
}
