//! The fourth wave's mount and pillager drills (research/parity/LAST-BRIEF.md "Group 1"): what
//! `slot.armor.body` takes on mounts, tame or wild, and on other mobs (`bodyslot`), and when held
//! pillagers swap their prey to a nearer actor and back (`pillswitch`).

use super::creatures::floor;

/// Packets shown of the body slot drill.
const BODY_SHOWN: &[&str] = &["AddEntity", "SetEntityData", "LevelSoundEvent", "MobArmorEquipment", "MobEquipment", "AddItemEntity"];

/// Packets shown of the prey drill: the pillagers' prey and the actor's moves.
const PREY_SHOWN: &[&str] = &["AddEntity", "SetEntityData", "MovePlayer", "EntityEvent"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "bodyslot" => (body_slot(), BODY_SHOWN),
        "pillswitch" => (pill_switch(), PREY_SHOWN),
        _ => return None,
    })
}

/// Each case: a kind, its spawn event, and what `/replaceitem` puts on it in order.
const BODY_CASES: &[(&str, Option<&str>, &[&str])] = &[
    ("horse", Some("spawn_tame_adult"), &["slot.armor.body 0 iron_horse_armor"]),
    ("horse", Some("spawn_tame_adult"), &["slot.armor 0 golden_horse_armor", "slot.armor.body 0 iron_horse_armor"]),
    ("horse", Some("spawn_tame_adult"), &["slot.armor.body 0 iron_horse_armor", "slot.armor 0 golden_horse_armor", "slot.armor 0 air"]),
    ("horse", Some("spawn_adult"), &["slot.armor.body 0 iron_horse_armor", "slot.armor 0 golden_horse_armor"]),
    ("horse", Some("spawn_adult"), &["slot.armor.body 0 stone", "slot.armor.body 0 diamond_helmet", "slot.armor.body 0 white_carpet", "slot.armor.body 0 air"]),
    ("zombie_horse", Some("spawn_tame_adult"), &["slot.armor.body 0 iron_horse_armor"]),
    ("donkey", Some("spawn_tame_adult"), &["slot.armor.body 0 iron_horse_armor"]),
    ("mule", Some("spawn_tame_adult"), &["slot.armor.body 0 iron_horse_armor"]),
    ("camel", Some("spawn_adult"), &["slot.armor.body 0 iron_horse_armor"]),
    ("llama", Some("spawn_adult"), &["slot.armor.body 0 white_carpet", "slot.armor.body 0 iron_horse_armor"]),
    ("llama", Some("spawn_tame_adult"), &["slot.armor.body 0 white_carpet", "slot.armor.body 0 iron_horse_armor"]),
    ("trader_llama", None, &["slot.armor.body 0 white_carpet"]),
    ("pig", None, &["slot.armor.body 0 iron_horse_armor"]),
    ("zombie", None, &["slot.armor.body 0 iron_horse_armor", "slot.armor.body 0 diamond_chestplate"]),
    ("wolf", None, &["slot.armor.body 0 iron_horse_armor", "slot.armor.body 0 wolf_armor"]),
];

/// Each case alone on the creature floor: summoned and held, its items put one a line, then
/// killed with `doMobLoot` on and its drops shown.
fn body_slot() -> Vec<String> {
    let mut lines = floor();
    lines.extend(["gamerule domobloot true", "difficulty peaceful", ""].map(String::from));
    for (kind, event, puts) in BODY_CASES {
        let event = event.map_or(String::new(), |event| format!(" 0 0 minecraft:{event}"));
        lines.extend([format!("summon {kind} 4.5 -26 0.5{event}"), format!("effect @e[type={kind}] slowness 200 255 true"), String::new(), String::new()]);
        for put in *puts {
            lines.extend([format!("replaceitem entity @e[type={kind}] {put}"), String::new()]);
        }
        lines.extend([format!("kill @e[type={kind}]"), String::new(), String::new(), "kill @e[type=item]".into(), String::new()]);
    }
    lines.push("difficulty easy".into());
    lines
}

/// Four pillagers slowed before any prey is near, then a slowed villager 6 west of them, the
/// survival actor 30 off: they take the villager. The actor comes 3 from them (nearer than the villager), leaves to 30, comes back;
/// the villager is killed with the actor away, then the actor comes back once more.
fn pill_switch() -> Vec<String> {
    let mut out: Vec<String> = [
        "kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "gamerule doweathercycle false", "weather clear", "time set 1000", "difficulty easy",
        "fill -8 -61 -8 44 -61 8 grass_block", "fill -8 -60 -8 44 -52 8 air", "tp CmdWatcher -4.5 -60 6.5", "gamemode creative CmdWatcher",
        "tp @s 40.5 -60 0.5", "gamemode survival @s", "clear @s", "effect @s resistance 1000 4 true", "effect @s saturation 1000 0 true",
        "summon pillager 10.5 -60 -1.5", "summon pillager 10.5 -60 -0.5", "summon pillager 10.5 -60 1.5", "summon pillager 10.5 -60 2.5",
        "effect @e[type=pillager] slowness 1000 255 true", "effect @e[type=pillager] resistance 1000 4 true",
        "summon villager 4.5 -60 0.5 0 0 minecraft:become_unskilled", "effect @e[type=villager_v2] slowness 1000 255 true", "effect @e[type=villager_v2] resistance 1000 4 true", "", "", "",
    ]
    .map(String::from)
    .into();
    let hold = |out: &mut Vec<String>, at: &str| out.extend([format!("tp @s {at}"), String::new(), format!("tp @s {at}"), String::new()]);
    hold(&mut out, "13.5 -60 0.5");
    hold(&mut out, "40.5 -60 0.5");
    hold(&mut out, "13.5 -60 0.5");
    hold(&mut out, "40.5 -60 0.5");
    out.extend(["kill @e[type=villager_v2]".to_owned(), String::new(), String::new()]);
    hold(&mut out, "14.5 -60 0.5");
    out.extend(["kill @e[type=pillager]", "kill @e[type=arrow]", "effect @s clear", "gamemode creative @s", ""].map(String::from));
    out
}
