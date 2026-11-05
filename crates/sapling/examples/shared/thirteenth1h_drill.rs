//! The thirteenth wave's group H drills (research/parity/THIRTEENTH-BRIEF.md): a summon's pitch and
//! summoned riders' name tags (`h1summon`), ticking areas in the Nether and the `/camera` forms
//! (`h1cmds`), `can_place_on` / `can_destroy` stacks in adventure mode (`h1lock`), and `/schedule`
//! against the probe pack of tools/bds-packs-setup.sh (`h1sched`, its BDS copy with KEEP_WORLD=1).

use super::creatures::{floor, waits, with};

const SUMMON_SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "MoveEntityDelta", "SetEntityLink", "CommandOutput"];
const CMDS_SHOWN: &[&str] = &["CameraInstruction", "CameraPresets", "CommandOutput"];
const LOCK_SHOWN: &[&str] = &["InventorySlot", "InventoryContent", "UpdateBlock", "LevelEvent", "CommandOutput"];
const SCHED_SHOWN: &[&str] = &["CommandOutput", "Text"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "h1summon" => (summon(), SUMMON_SHOWN),
        "h1cmds" => (commands(), CMDS_SHOWN),
        "h1lock" => (lock(), LOCK_SHOWN),
        "h1sched" => (schedule(), SCHED_SHOWN),
        "h1camera" => (camera(), CMDS_SHOWN),
        _ => return None,
    })
}

/// A pig summoned facing 90 and pitched 45 left to look around; riders and rides summoned with a
/// name, onto a pig that refuses a zombie and onto the actor.
fn summon() -> Vec<String> {
    let mut lines = with(floor(), &["summon pig 2 -26 2 90 45"]);
    waits(&mut lines, 5);
    lines.extend(["ride @e[type=pig] summon_rider zombie minecraft:entity_spawned Rider", "", "", "summon minecart 6 -26 -3", "", "ride @e[type=minecart] summon_rider skeleton minecraft:entity_spawned Bones", "", "ride @s summon_ride horse no_ride_change minecraft:entity_spawned Ride", "", "ride @s stop_riding", ""].map(String::from));
    with(lines, &["kill @e[type=!player]", ""])
}

/// Ticking areas in both dimensions listed, and the `/camera` forms group F did not build.
fn commands() -> Vec<String> {
    with(floor(), &[
        "help camera",
        "help place",
        "help schedule",
        "execute in nether run tickingarea add 0 0 0 0 0 0 h1nether",
        "tickingarea add 0 0 0 0 0 0 h1over",
        "tickingarea list all-dimensions",
        "tickingarea list",
        "execute in nether run tickingarea list",
        "execute in nether run tickingarea list all-dimensions",
        "tickingarea remove_all",
        "execute in nether run tickingarea remove_all",
        "summon pig 3 -26 0",
        "",
        "camera @s attach_to_entity @e[type=pig,c=1]",
        "camera @s detach_from_entity",
        "camera @s target_entity @e[type=pig,c=1]",
        "camera @s target_entity @e[type=pig,c=1] target_center_offset 0 1 0",
        "camera @s remove_target",
        "camera @s set minecraft:third_person view_offset 1 2",
        "camera @s set minecraft:third_person entity_offset 1 2 3",
        "camera @s set minecraft:third_person view_offset 1 2 entity_offset 1 2 3",
        "camera @s attach_to_entity @e[type=cow]",
        "camera @s target_entity @e[type=cow]",
        "camera @s clear",
        "kill @e[type=!player]",
        "",
    ])
}

/// The `/camera` forms h1cmds refused, under the presets that may take them.
fn camera() -> Vec<String> {
    let mut lines = with(floor(), &["summon pig 3 -26 0", ""]);
    for preset in ["minecraft:follow_orbit", "minecraft:free", "minecraft:fixed_boom", "minecraft:first_person"] {
        lines.extend([
            format!("camera @s set {preset}"),
            "camera @s target_entity @e[type=pig,c=1]".into(),
            "camera @s target_entity @e[type=pig,c=1] target_center_offset 0 1 0".into(),
            format!("camera @s set {preset} view_offset 1 2"),
            format!("camera @s set {preset} entity_offset 1 2 3"),
            format!("camera @s set {preset} ease 1 linear rot 10 20 view_offset 1 2 entity_offset 1 2 3"),
        ]);
    }
    with(lines, &[
        "camera @s set minecraft:follow_orbit view_offset 40 2",
        "camera @s set minecraft:follow_orbit entity_offset 1 200 3",
        "camera @s target_entity @a",
        "camera @s target_entity @e[type=cow]",
        "camera @s attach_to_entity @e",
        "camera @s play_spline nosuch",
        "camera @s play_spline minecraft:nosuch",
        "camera @s remove_target",
        "camera @s clear",
        "kill @e[type=!player]",
        "",
    ])
}

/// Stacks that may be placed only on grass, or break only dirt, given and used in adventure mode.
fn lock() -> Vec<String> {
    with(floor(), &[
        "setblock 2 -27 0 grass_block",
        "setblock 2 -26 2 dirt",
        "setblock 3 -26 2 sand",
        r#"give @s stone 4 0 {"minecraft:can_place_on":{"blocks":["grass_block"]}}"#,
        r#"give @s diamond_pickaxe 1 0 {"minecraft:can_destroy":{"blocks":["dirt","minecraft:stone"]}}"#,
        r#"give @s dirt 1 0 {"minecraft:can_place_on":{"blocks":["nosuch"]}}"#,
        r#"give @s dirt 1 0 {"minecraft:can_place_on":{"blocks":"stone"}}"#,
        r#"give @s dirt 1 0 {"can_place_on":{"blocks":["stone"]}}"#,
        "gamemode adventure @s",
        "",
        "!place 2 -1 0",
        "",
        "!place 3 -1 0",
        "",
        "replaceitem entity @s slot.hotbar 0 diamond_pickaxe",
        r#"replaceitem entity @s slot.hotbar 0 diamond_pickaxe 1 0 {"minecraft:can_destroy":{"blocks":["dirt"]}}"#,
        "",
        "!break 2 0 2",
        "",
        "!break 3 0 2",
        "",
        "gamemode creative @s",
        "clear @s",
        "fill -8 -26 -8 16 -16 8 air",
        "setblock 2 -27 0 stone",
        "",
    ])
}

/// `/schedule` with no mode twice, then the due tick against the level's.
fn schedule() -> Vec<String> {
    with(Vec::new(), &[
        "scoreboard objectives add fn dummy",
        "time query gametime",
        "schedule delay add probe/hello 20",
        "schedule delay add probe/hello 20",
        "",
        "",
        "schedule delay add probe/hello 30",
        "schedule delay add probe/hello 20 append",
        "",
        "",
        "scoreboard players list @s",
        "scoreboard objectives remove fn",
    ])
}
