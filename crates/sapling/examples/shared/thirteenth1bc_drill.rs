//! The thirteenth wave's group BC drills (research/parity/THIRTEENTH-BRIEF.md). On the flat ground
//! (stand at y -60): `/event entity` of sheep wool, colour and growing events, a baby turtle grown by its
//! event, and the player's raid omen events, to see what each changes and when it is told against the
//! answer (`bc1event`). On the drill floor at y -27 at midnight: a zombie hunting the survival actor
//! across a row of low blocks (a slab, a daylight detector, a turtle egg, a stonecutter, an enchanting
//! table), each watched 12 s, to see where it steps up and where it jumps (`bc1step`).

use crate::villages::wait;

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

fn events() -> Vec<String> {
    let mut out = owned([
        "kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "time set 1000", "tp @a 0.5 -60 0.5", "",
        "summon sheep 2.5 -60 4.5", "summon sheep 4.5 -60 4.5", "summon sheep 6.5 -60 4.5 0 0 minecraft:entity_born", "effect @e[type=sheep] slowness 1000 255 true", "", "",
        "event entity @e[type=sheep] wololo", "",
        "event entity @e[type=sheep] minecraft:on_sheared", "",
        "event entity @e[type=sheep] minecraft:on_sheared", "",
        "event entity @e[type=sheep] minecraft:on_eat_block", "",
        "event entity @e[type=sheep] minecraft:cold_color", "",
        "event entity @e[type=sheep] minecraft:warm_color", "",
        "event entity @e[type=sheep] minecraft:ageable_grow_up", "",
        "summon turtle 8.5 -60 4.5 0 0 minecraft:entity_born", "", "",
        "event entity @e[type=turtle] minecraft:ageable_grow_up", "", "",
        "effect @s bad_omen 100 2", "",
        "event entity @s minecraft:gain_raid_omen", "",
        "event entity @s minecraft:clear_add_raid_omen", "",
        "event entity @s minecraft:trigger_raid", "",
        "event entity @s minecraft:remove_raid_trigger", "",
        "effect @s clear",
        "kill @e[type=!player]", "",
    ]);
    wait(&mut out, 2);
    out
}

/// What `bc1step` lays across the zombie's way, two blocks before the actor's floor ends.
const LOW: [&str; 5] = ["smooth_stone_slab", "daylight_detector", "turtle_egg", "stonecutter_block", "enchanting_table"];

fn steps() -> Vec<String> {
    let mut out = super::warden::start("midnight");
    let reset = ["kill @e[type=!player]", "fill -20 -26 -20 20 -16 0 air", "fill -20 -26 0 20 -16 20 air", "tp @s 0.5 -26 0.5"];
    for block in LOW {
        out.extend(reset.map(String::from));
        out.push(format!("fill -20 -26 -6 20 -26 -6 {block}"));
        out.push("summon zombie 0.5 -26 -11.5".into());
        super::warden::healed(&mut out, 12);
    }
    out.extend(reset.map(String::from));
    out.extend(owned(["gamemode creative @s", "effect @s clear"]));
    out
}

const EVENT_SHOWN: &[&str] = &["CommandOutput", "SetEntityData", "MobEffect", "AddEntity", "AddItemEntity", "RemoveEntity", "EntityEvent", "LevelEvent", "LevelSoundEvent"];
const STEP_SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityMotion", "CommandOutput", "EntityEvent", "UpdateBlock"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "bc1event" => (events(), EVENT_SHOWN),
        "bc1step" => (steps(), STEP_SHOWN),
        _ => return None,
    })
}
