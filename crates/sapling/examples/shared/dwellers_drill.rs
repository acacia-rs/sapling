//! The still-open raid, villager and enchanting drills of bds_command_probe (docs/research/
//! effects-raids.md, villagers.md, leftovers.md): which prey a raid's pillagers take when held
//! nearer the villagers or the actor, and whether they keep hunting the actor after the village
//! falls (`raidaim`); the `ench` order `/enchant` leaves on swords, armour and bows (`enchorder`);
//! the village drills' setup alone, to time its fills (`villageset`).
//! Places are absolute on the flat ground; drill sapling with `SAPLING_DRILL_WORLD=lowflat`.

use crate::villages::{UNSKILLED, VILLAGE_SHOWN, every, village, wait};

const NEAR: &str = "testfor @e[family=monster,x=0,y=-60,z=6,r=16]";

fn lines<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

/// The first wave held still 10 from the slowed villagers (14.6 from the actor), then 12 from the
/// actor (15.5 from the nearest villager); then the villagers killed, the pillagers watched 20 s.
fn raid_aim() -> Vec<String> {
    let mut out = village(&UNSKILLED);
    out.extend(lines(["effect @e[type=villager_v2] resistance 1000 4 true", "effect @e[type=villager_v2] slowness 1000 255 true", "effect @s bad_omen 600 0"]));
    wait(&mut out, 52);
    out.extend(lines(["effect @e[type=pillager] slowness 1000 255 true", "tp @e[type=pillager] -3.5 -60 14.5", NEAR]));
    wait(&mut out, 10);
    out.extend(lines(["tp @e[type=pillager] 12.5 -60 0.5", NEAR]));
    wait(&mut out, 10);
    out.push("kill @e[type=villager_v2]".into());
    every(&mut out, NEAR, 3, 7);
    out.extend(lines(["kill @e[family=monster]", "effect @s clear", "gamemode creative @s", ""]));
    out
}

/// `enchants` given one by one, a level raised among them, then a resend so the item shows.
fn enchanted(item: &str, enchants: &[&str]) -> Vec<String> {
    let mut out = vec!["clear @s".to_owned(), format!("replaceitem entity @s slot.hotbar 0 {item}"), String::new()];
    out.extend(enchants.iter().map(|enchant| format!("enchant @s {enchant}")));
    out.extend(lines(["", "replaceitem entity @s slot.hotbar 8 stick", "", "!inv"]));
    out
}

fn ench_order() -> Vec<String> {
    let mut out = crate::cmdblocks::floor();
    out.extend(enchanted("diamond_sword", &["vanishing 1", "knockback 1", "sharpness 1", "unbreaking 1", "sharpness 3", "looting 2", "mending 1"]));
    out.extend(enchanted("diamond_chestplate", &["mending 1", "unbreaking 1", "protection 2", "thorns 1", "binding 1", "protection 4"]));
    out.extend(enchanted("bow", &["power 1", "infinity 1", "punch 1", "flame 1", "unbreaking 2", "power 3"]));
    out.extend(enchanted("diamond_boots", &["feather_falling 1", "soul_speed 1", "depth_strider 1", "unbreaking 1", "mending 1"]));
    out.push("clear @s".into());
    out
}

/// The lines and shown packets of the drill `name`, if it is one of these.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "raidaim" => (raid_aim(), VILLAGE_SHOWN),
        "enchorder" => (ench_order(), crate::crafting::SHOWN),
        "villageset" => (village(&UNSKILLED), VILLAGE_SHOWN),
        _ => return None,
    })
}
