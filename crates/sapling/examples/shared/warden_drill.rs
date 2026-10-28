//! The warden and creaking heart drills of bds_command_probe. In fixed places, as wander_drill.rs:
//! a stone floor 41 across at y -27, the actor in survival with ten more hearts, healed every second.

/// Packets shown of these drills and of props_drill.rs's.
pub const SHOWN: &[&str] = &[
    "UpdateAttributes", "EntityEvent", "SetEntityData", "SetHealth", "LevelSoundEvent", "LevelEvent", "MovePlayer", "SetEntityMotion", "Animate", "RemoveEntity", "AddEntity", "AddItemEntity",
    "AddPainting", "MoveEntityDelta", "MoveEntityAbsolute", "CorrectPlayerMovePrediction", "MobEquipment", "MobArmorEquipment", "MobEffect", "UpdateBlock", "BlockEntityData", "SetEntityLink",
    "TakeItemEntity", "InventorySlot",
];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some((
        match name {
            "warden" => warden(),
            "heart" => heart(),
            _ => return None,
        },
        SHOWN,
    ))
}

pub fn start(time: &str) -> Vec<String> {
    [
        "kill @e[type=!player]",
        "gamerule domobspawning false",
        "weather clear",
        "difficulty easy",
        "gamemode creative CmdWatcher",
        "effect @s clear",
        "clear @s",
        "fill -20 -27 -20 20 -27 20 stone",
        "fill -20 -26 -20 20 -16 0 air",
        "fill -20 -26 0 20 -16 20 air",
        &format!("time set {time}"),
        "tp CmdWatcher 14.5 -26 14.5",
        "tp @s 0.5 -26 0.5",
        "gamemode survival @s",
        "effect @s health_boost 900 4",
        "effect @s instant_health 1 9",
        "",
    ]
    .map(String::from)
    .into()
}

pub fn healed(lines: &mut Vec<String>, seconds: usize) {
    lines.extend(std::iter::repeat_n("effect @s instant_health 1 4".to_owned(), seconds));
}

/// A warden 10 off the still actor (sniffing, outside its 6 of suspicion), the actor jumping 8 off
/// (vibrations: anger, roar, the chase and hits), the actor in a glass cage on a pillar in a
/// diamond chestplate (sonic booms), then one summoned emerging and left alone past its 60 s idle
/// (digging back down).
pub fn warden() -> Vec<String> {
    let mut lines = start("noon");
    lines.extend(["tp @s 0.5 -26 0.5 facing 0.5 -25 -8", "summon warden 0.5 -26 -9.5"].map(String::from));
    healed(&mut lines, 20);
    lines.extend(["tp @s 0.5 -26 -1.5 facing 0.5 -25 -8", "!jump"].map(String::from));
    healed(&mut lines, 6);
    lines.push("!stop".into());
    healed(&mut lines, 14);
    lines.extend(
        ["fill 5 -26 5 7 -18 7 glass", "fill 6 -21 6 6 -20 6 air", "tp @s 6.5 -21 6.5", "replaceitem entity @s slot.armor.chest 0 diamond_chestplate"].map(String::from),
    );
    healed(&mut lines, 20);
    lines.extend(["kill @e[type=warden]", "gamemode creative @s", "clear @s", "fill 5 -26 5 7 -18 7 air", "tp @s 0.5 -26 0.5", "", "summon warden 8.5 -26 0.5 0 0 minecraft:spawn_emerging"].map(String::from));
    lines.extend(std::iter::repeat_n(String::new(), 80));
    lines.extend(["effect @s clear", "kill @e[type=!player]", ""].map(String::from));
    lines
}

/// At midnight a natural creaking heart between two pale oak logs 6 off, the actor facing away:
/// its creaking comes; faced, the actor hits it three times; the heart is broken under it. Then a
/// second heart whose creaking is left to the morning.
pub fn heart() -> Vec<String> {
    let mut lines = start("midnight");
    let build = ["setblock 6 -26 0 pale_oak_log", "setblock 6 -25 0 creaking_heart [\"natural\"=true]", "setblock 6 -24 0 pale_oak_log", "tp @s 0.5 -26 0.5 facing -8 -25 0.5"];
    lines.extend(build.map(String::from));
    healed(&mut lines, 20);
    for _ in 0..3 {
        lines.extend(["tp @s ~ ~ ~ facing @e[type=creaking,c=1]", "!hit creaking", "effect @s instant_health 1 4"].map(String::from));
    }
    healed(&mut lines, 2);
    lines.extend(["setblock 6 -25 0 air destroy", ""].map(String::from));
    healed(&mut lines, 4);
    lines.extend(["kill @e[type=!player]", "fill 6 -26 0 6 -24 0 air", "setblock 6 -26 0 pale_oak_log", "setblock 6 -25 0 creaking_heart [\"natural\"=true]", "setblock 6 -24 0 pale_oak_log", "tp @s 0.5 -26 0.5 facing -8 -25 0.5"].map(String::from));
    healed(&mut lines, 15);
    lines.push("time set noon".into());
    healed(&mut lines, 8);
    lines.extend(["effect @s clear", "gamemode creative @s", "kill @e[type=!player]", "fill 6 -26 0 6 -24 0 air", ""].map(String::from));
    lines
}
