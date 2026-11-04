//! The twelfth wave's group 1 drills (research/parity/TWELFTH-BRIEF.md): cave spiders biting the
//! survival actor on easy and on hard (`t12cave`, `t12bites` with a wither skeleton on easy); a
//! chested donkey's goods with `doMobLoot` off (`t12goods`); a 513-letter book page (`t12book`).

use super::creatures::{floor, waits, with};

const SHOWN: &[&str] = &[
    "AddEntity", "RemoveEntity", "AddItemEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityMotion", "SetEntityData", "EntityEvent", "LevelSoundEvent", "LevelEvent", "MobEffect",
    "UpdateAttributes", "SetHealth", "MobEquipment", "MobArmorEquipment", "UpdateBlock", "BlockEntityData", "CommandOutput", "Text",
];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "t12cave" => (cave_spiders(), SHOWN),
        "t12goods" => (goods(), SHOWN),
        "t12bites" => (bites(), SHOWN),
        "t12book" => (book(), SHOWN),
        _ => return None,
    })
}

/// At midnight, the survival actor (healed): a cave spider 6 off on easy, watched 12 s, then one on
/// hard.
fn cave_spiders() -> Vec<String> {
    let mut lines = super::warden::start("midnight");
    for level in ["easy", "hard"] {
        lines.extend([format!("difficulty {level}"), "summon cave_spider 6.5 -26 0.5".into()]);
        super::warden::healed(&mut lines, 12);
        lines.extend(["kill @e[type=!player]", "effect @s clear", "effect @s health_boost 900 4", "effect @s instant_health 1 9", "tp @s 0.5 -26 0.5", ""].map(String::from));
    }
    with(lines, &["difficulty easy", "gamemode creative @s", "effect @s clear"])
}

/// At midnight, the survival actor (healed): two cave spiders 6 off on hard, watched 25 s, then a
/// wither skeleton on easy, watched 15 s.
fn bites() -> Vec<String> {
    let mut lines = super::warden::start("midnight");
    lines.extend(["difficulty hard", "summon cave_spider 6.5 -26 0.5", "summon cave_spider -5.5 -26 0.5"].map(String::from));
    super::warden::healed(&mut lines, 25);
    lines.extend(["kill @e[type=!player]", "effect @s clear", "effect @s health_boost 900 4", "effect @s instant_health 1 9", "tp @s 0.5 -26 0.5", "difficulty easy", "summon wither_skeleton 6.5 -26 0.5"].map(String::from));
    super::warden::healed(&mut lines, 15);
    with(lines, &["kill @e[type=!player]", "gamemode creative @s", "effect @s clear"])
}

/// A book of one page, its page then sent raw as 513 letters (512 was kept, 1000 got the writer
/// kicked: bds-t2ebook.txt, bds-t2ebook1.txt), the stack listed after.
fn book() -> Vec<String> {
    let page = format!("!rawpage {}", "d".repeat(513));
    with(floor(), &["replaceitem entity @s slot.weapon.mainhand 0 writable_book", "", "!write x", "", &page, "", "!inv", "clear @s"])
}

/// A tamed donkey given a chest by a click, then a saddle and goods by command, killed with
/// `doMobLoot` off; the items listed after.
fn goods() -> Vec<String> {
    let mut lines = with(floor(), &["time set noon", "gamerule domobloot false", "summon donkey 2.5 -26 0.5 0 0 minecraft:spawn_tame_adult", "effect @e[type=donkey] slowness 200 255 true", "", "clear @s", "give @s chest", "tp @s 0.5 -26 0.5 -90 0", "", "!interact donkey", ""]);
    lines.extend(["saddle 0 saddle", "inventory 0 diamond 5", "inventory 2 emerald 2"].map(|slot| format!("replaceitem entity @e[type=donkey] slot.{slot}")));
    lines.extend(["", "kill @e[type=donkey]"].map(String::from));
    waits(&mut lines, 3);
    with(lines, &["testfor @e[type=item]", "kill @e[type=item]", "gamerule domobloot true", "kill @e[type=!player]"])
}
