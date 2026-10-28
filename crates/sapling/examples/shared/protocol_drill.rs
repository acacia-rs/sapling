//! Drills of bds_command_probe for packets sapling did not send: `properties`, the entity
//! properties (`description.properties` of the vanilla packs) each kind that has them is announced
//! with, over repeated summons, and a bee's `has_nectar` as it takes pollen, printed in
//! [`properties`]'s short form; `hints`, what moves orbs, mobs, items and a cart and what ends a
//! bundle, for MotionPredictionHints and ContainerRegistryCleanup; `hints2`, mobs falling and
//! hopping down steps; `cleanup`, bundles cleared and watched for when their containers go.

use acacia_bot::proto::{Packet, RawPacket, packets};

/// `Properties` asks for [`properties`]'s form of AddEntity and SetEntityData.
pub const SHOWN: &[&str] = &["Properties", "RemoveEntity"];
pub const HINTS_SHOWN: &[&str] = &["MotionPredictionHints", "ContainerRegistryCleanup", "AddEntity", "AddItemEntity", "RemoveEntity", "TakeItemEntity", "ItemStackResponse"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    match name {
        "properties" => Some((properties_drill(), SHOWN)),
        "hints" => Some((hints(), HINTS_SHOWN)),
        "hints2" => Some((hints_mobs(), HINTS2_SHOWN)),
        "cleanup" => Some((cleanup(), HINTS2_SHOWN)),
        _ => None,
    }
}

const HINTS2_SHOWN: &[&str] = &["Properties", "MotionPredictionHints", "ContainerRegistryCleanup", "RemoveEntity"];

/// Mobs that fall, hop, jump and are thrown on a stepped floor, each left to itself for a while.
fn hints_mobs() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "gamerule domobspawning false", "time set noon", "gamemode creative @s", "fill -12 -27 -12 12 -27 12 stone", "fill -12 -26 -12 12 -16 12 air"]
        .map(String::from)
        .into();
    lines.extend((0..6).map(|step| format!("fill {} -26 -12 {} {} 12 grass_block", step * 2, step * 2 + 1, -27 + step)));
    lines.extend(["tp @s -6.5 -26 0.5 90 0", "tp CmdWatcher -6.5 -26 4.5", ""].map(String::from));
    for kind in ["cow", "sheep", "rabbit", "slime", "wolf", "frog", "goat", "horse"] {
        lines.push(format!("summon {kind} 3.5 -16 0.5"));
        waits(&mut lines, 6);
    }
    lines.extend(["!hit cow", "", "!hit sheep", ""].map(String::from));
    waits(&mut lines, 20);
    lines.push("kill @e[type=!player]".into());
    lines
}

/// Bundles lost by `/clear` at different times, each watched for long enough to see when BDS
/// says their containers are gone.
fn cleanup() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "gamemode survival @s", "clear @s", ""].map(String::from).into();
    for wait in [7, 11] {
        lines.extend(["replaceitem entity @s slot.hotbar 0 bundle 1", "replaceitem entity @s slot.hotbar 1 cobblestone 64", "", "!bundletag", "!bundleput 1 10 0", ""].map(String::from));
        waits(&mut lines, wait);
        lines.push("clear @s bundle".into());
        waits(&mut lines, 34);
    }
    lines.extend(["gamemode creative @s", "clear @s"].map(String::from));
    lines
}

fn waits(lines: &mut Vec<String>, seconds: usize) {
    lines.extend(std::iter::repeat_n(String::new(), seconds));
}

/// Orbs by a creative actor (left lying) and a survival one (drawn in and taken), a cow and a pig
/// walking and the pig hit, an item thrown, a cart pushed off a slope; then a bundle with
/// cobblestone in it cleared, another thrown and killed lying, and a third replaced by air.
fn hints() -> Vec<String> {
    let mut lines: Vec<String> = [
        "kill @e[type=!player]",
        "gamerule domobspawning false",
        "weather clear",
        "time set noon",
        "gamemode creative @s",
        "gamemode creative CmdWatcher",
        "clear @s",
        "fill -12 -27 -12 12 -27 12 stone",
        "fill -12 -26 -12 12 -16 12 air",
        "tp @s 0.5 -26 0.5 -90 0",
        "tp CmdWatcher 0.5 -26 4.5",
        "",
    ]
    .map(String::from)
    .into();
    lines.extend((0..6).map(|at| format!("summon xp_orb {}.5 -25 -3.5", at - 3)));
    waits(&mut lines, 10);
    lines.extend(["gamemode survival @s", "summon xp_orb 2.5 -25 0.5", "summon xp_orb 3.5 -25 0.5"].map(String::from));
    waits(&mut lines, 8);
    lines.extend(["gamemode creative @s", "kill @e[type=!player]", "summon cow 4.5 -26 -6.5", "summon pig 2.5 -26 0.5"].map(String::from));
    waits(&mut lines, 8);
    lines.extend(["!hit pig", "", "", "", "kill @e[type=!player]", "replaceitem entity @s slot.hotbar 5 cobblestone 8", "!drop 5 1"].map(String::from));
    waits(&mut lines, 4);
    lines.extend(["fill 5 -26 -2 5 -26 2 stone", "summon minecart 5.5 -25 0.5", "summon minecart 4.5 -26 0.5"].map(String::from));
    waits(&mut lines, 6);
    lines.extend(["kill @e[type=!player]", "gamemode survival @s", "clear @s"].map(String::from));
    let bundle = |lines: &mut Vec<String>| lines.extend(["replaceitem entity @s slot.hotbar 0 bundle 1", "replaceitem entity @s slot.hotbar 1 cobblestone 64", "", "!bundletag", "!bundleput 1 10 0", ""].map(String::from));
    bundle(&mut lines);
    lines.extend(["clear @s bundle", "", ""].map(String::from));
    bundle(&mut lines);
    lines.extend(["!drop 0 1", "", "kill @e[type=item]", "", ""].map(String::from));
    bundle(&mut lines);
    lines.extend(["replaceitem entity @s slot.hotbar 0 air", "", "", "gamemode creative @s", "clear @s"].map(String::from));
    lines
}

/// Kinds summoned, and how often: those whose properties a dice picks, more times.
const KINDS: &[(&str, usize)] = &[
    ("pig", 8),
    ("cow", 6),
    ("chicken", 6),
    ("wolf", 8),
    ("cat", 6),
    ("armadillo", 1),
    ("happy_ghast", 1),
    ("copper_golem", 1),
    ("creaking", 1),
    ("zombie_nautilus", 1),
    ("sulfur_cube", 1),
];

fn properties_drill() -> Vec<String> {
    let mut lines: Vec<String> = [
        "kill @e[type=!player]",
        "gamerule domobspawning false",
        "gamerule domobloot false",
        "weather clear",
        "time set noon",
        "gamemode creative @s",
        "gamemode creative CmdWatcher",
        "fill -12 -27 -12 12 -27 12 stone",
        "fill -12 -26 -12 12 -16 12 air",
        "tp @s 0.5 -26 0.5 -90 0",
        "tp CmdWatcher 0.5 -26 0.5",
        "",
    ]
    .map(String::from)
    .into();
    for (row, (kind, times)) in KINDS.iter().enumerate() {
        for at in 0..*times {
            lines.push(format!("summon {kind} {} -26 {}", at as i32 - 4, row as i32 - 6));
        }
        lines.push(String::new());
    }
    lines.extend(["kill @e[type=!player]", "", "fill 3 -27 4 4 -27 4 grass_block", "setblock 3 -26 4 poppy", "setblock 4 -26 4 dandelion", "summon bee 4.5 -25 2.5"].map(String::from));
    lines.extend(std::iter::repeat_n(String::new(), 30));
    lines.push("kill @e[type=!player]".into());
    lines
}

/// AddEntity and SetEntityData as `<packet> <id> [<kind>] ints=[(index, value)..] floats=[..]`
/// and the metadata keys a SetEntityData carries; `None` for other packets.
pub fn properties(packet: &RawPacket) -> Option<String> {
    let pairs = |ints: &[acacia_bot::proto::types::EntityPropertiesIntsItem]| ints.iter().map(|int| format!("({},{})", int.index, int.value)).collect::<Vec<_>>().join(" ");
    let floats = |floats: &[acacia_bot::proto::types::EntityPropertiesFloatsItem]| floats.iter().map(|float| format!("({},{})", float.index, float.value)).collect::<Vec<_>>().join(" ");
    if packet.id == packets::AddEntity::ID {
        let add = packet.decode::<packets::AddEntity>().ok()?;
        return Some(format!("AddEntity {} {} ints=[{}] floats=[{}]", add.runtime_id, add.entity_type, pairs(&add.properties.ints), floats(&add.properties.floats)));
    }
    if packet.id == packets::ContainerRegistryCleanup::ID {
        let body: String = packet.body.iter().map(|byte| format!("{byte:02x}")).collect();
        return Some(format!("ContainerRegistryCleanup {body} {:?}", packet.decode::<packets::ContainerRegistryCleanup>().ok()?.removed_containers));
    }
    if packet.id == packets::SetEntityData::ID {
        let set = packet.decode::<packets::SetEntityData>().ok()?;
        let keys: Vec<String> = set.metadata.iter().map(|item| format!("{:?}={:?}", item.key, item.value)).collect();
        return Some(format!("SetEntityData {} ints=[{}] floats=[{}] metadata=[{}]", set.runtime_entity_id, pairs(&set.properties.ints), floats(&set.properties.floats), keys.join(", ")));
    }
    None
}
