//! Fourteenth wave, group AA (drill2): `aa2spear`: a zombie holding an iron spear charges the
//! survival actor standing still on a dirt field, on easy then on normal, then one with a wooden spear.

use crate::villages::wait;

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

fn spears() -> Vec<String> {
    let mut out = owned(["kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "gamerule falldamage false", "weather clear", "time set 6000"]);
    out.extend(owned(["gamemode creative @a", "effect @a clear", "clear @s", "fill -16 -60 -16 16 -54 16 air", "fill -16 -61 -16 16 -61 16 grass", "tp CmdWatcher 0.5 -60 -40.5", ""]));
    for (difficulty, spear) in [("easy", "iron_spear"), ("normal", "iron_spear"), ("easy", "wooden_spear")] {
        out.extend([format!("difficulty {difficulty}"), "gamemode survival @s".into(), "effect @s health_boost 1000 9 true".into(), "effect @s instant_health 1 9 true".into()]);
        out.extend(owned(["tp @s 0.5 -60 0.5 0 0", "summon zombie 0.5 -60 10.5 180 0", "effect @e[type=zombie] slowness 2 255 true"]));
        out.push(format!("replaceitem entity @e[type=zombie] slot.weapon.mainhand 0 {spear}"));
        out.push(String::new());
        wait(&mut out, 25);
        out.extend(owned(["kill @e[type=zombie]", "gamemode creative @s", "effect @s clear", ""]));
    }
    out
}

const SHOWN: &[&str] = &[
    "CommandOutput", "AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityData", "EntityEvent", "SetEntityMotion", "LevelSoundEvent", "UpdateAttributes", "MobEquipment",
];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "aa2spear" => (spears(), SHOWN),
        _ => return None,
    })
}
