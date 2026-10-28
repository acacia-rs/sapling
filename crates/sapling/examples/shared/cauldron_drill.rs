//! Drills of cauldrons: potions poured in and taken out, dyed water, washing, and rain.

/// Packets shown of these drills.
pub const CAULDRON_SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "BlockEntityData", "InventorySlot", "LevelEvent", "LevelSoundEvent"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some((
        match name {
            "cauldrons" => cauldrons(),
            "cauldronrain" => rain(),
            "cauldrons2" => buckets_and_banners(),
            _ => return None,
        },
        CAULDRON_SHOWN,
    ))
}

fn floor() -> Vec<String> {
    ["kill @e[type=!player]", "gamerule domobspawning false", "weather clear", "gamemode creative @s", "tp @s 0.5 ~ 0.5 -90 0", "fill ~-2 ~-1 ~-6 ~10 ~-1 ~6 stone", "fill ~-2 ~ ~-6 ~10 ~6 ~6 air", "clear @s", ""]
        .map(String::from)
        .into()
}

fn hold(item: &str) -> String {
    format!("replaceitem entity @s slot.weapon.mainhand 0 {item}")
}

/// The actor in survival clicks a cauldron two east of it: swiftness poured in twice, slowness
/// on top, swiftness twice again and a bottle taken out, a splash potion of swiftness, arrows.
/// Then a full cauldron of water north-east: red dye, a leather cap dyed in it, then washed in a
/// full cauldron of water south-east, with a red shulker box and a plain banner after it.
pub fn cauldrons() -> Vec<String> {
    let mut lines = floor();
    lines.extend(["setblock ~2 ~ ~0 cauldron".to_owned(), "gamemode survival @s".to_owned(), String::new()]);
    for item in ["potion 1 14", "potion 1 14", "potion 1 17", "potion 1 14", "potion 1 14", "glass_bottle 1", "splash_potion 1 14", "arrow 16"] {
        lines.extend([hold(item), "!click 2 0 0".to_owned(), String::new()]);
    }
    lines.extend(["setblock ~2 ~ ~-3 cauldron [\"fill_level\"=6]".to_owned(), "setblock ~2 ~ ~3 cauldron [\"fill_level\"=6]".to_owned(), String::new()]);
    for (item, z) in [("red_dye 1", -3), ("leather_helmet 1", -3), ("", 3), ("red_shulker_box 1", 3), ("banner 1 1", 3)] {
        if !item.is_empty() {
            lines.push(hold(item));
        }
        lines.extend([format!("!click 2 0 {z}"), String::new()]);
    }
    lines.extend(["gamemode creative @s".to_owned(), "fill ~-2 ~ ~-6 ~10 ~6 ~6 air".to_owned(), "clear @s".to_owned(), String::new()]);
    lines
}

/// The actor in survival takes a bucket to a full cauldron of swiftness, one of two levels and a full
/// cauldron of water dyed red then blue, pours swiftness into plain water, then washes a banner
/// patterned at a loom in a full cauldron of water, twice.
pub fn buckets_and_banners() -> Vec<String> {
    let mut lines = floor();
    let set = ["setblock ~2 ~ ~0 cauldron", "setblock ~2 ~ ~-3 cauldron", "setblock ~2 ~ ~3 cauldron [\"fill_level\"=6]", "setblock ~5 ~ ~-3 cauldron [\"fill_level\"=6]", "setblock ~8 ~ ~0 cauldron [\"fill_level\"=3]", "setblock ~5 ~ ~3 loom"];
    lines.extend(set.into_iter().chain(["gamemode survival @s", ""]).map(String::from));
    let clicks = [
        ("potion 1 14", "2 0 0"),
        ("potion 1 14", "2 0 0"),
        ("potion 1 14", "2 0 0"),
        ("bucket 1", "2 0 0"),
        ("potion 1 14", "2 0 -3"),
        ("bucket 1", "2 0 -3"),
        ("red_dye 1", "5 0 -3"),
        ("blue_dye 1", "5 0 -3"),
        ("bucket 1", "5 0 -3"),
        ("potion 1 14", "8 0 0"),
    ];
    for (item, at) in clicks {
        lines.extend([hold(item), format!("!click {at}"), String::new()]);
    }
    lines.extend(["replaceitem entity @s slot.hotbar 0 banner 1 15", "replaceitem entity @s slot.hotbar 1 red_dye 1", "", "!loom 5 0 3 bo", "", "!close", "", "!click 2 0 3", "", "!click 2 0 3", ""].map(String::from));
    lines.extend(["gamemode creative @s", "fill ~-2 ~ ~-6 ~10 ~6 ~6 air", "clear @s", ""].map(String::from));
    lines
}

/// An empty cauldron under an open sky in the rain for a minute, at `randomtickspeed` 100.
pub fn rain() -> Vec<String> {
    let mut lines = floor();
    lines.extend(["setblock ~2 ~ ~0 cauldron", "gamerule randomtickspeed 100", "weather rain"].map(String::from));
    lines.extend(std::iter::repeat_n(String::new(), 60));
    lines.extend(["weather clear", "gamerule randomtickspeed 1", "setblock ~2 ~ ~0 air", ""].map(String::from));
    lines
}
