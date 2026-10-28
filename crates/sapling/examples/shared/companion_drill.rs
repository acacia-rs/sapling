//! Drills of mobs and what they hold or ride: a tamed parrot on the player's shoulder, drowned's
//! tridents dropped as they die, and a piglin's gold kept across a restart. On creature_drill.rs's floor.

use super::creatures::{floor, waits, with};

/// Packets shown of these drills.
pub const COMPANION_SHOWN: &[&str] = &[
    "AddEntity", "RemoveEntity", "SetEntityLink", "SetEntityData", "EntityEvent", "ActorEvent", "LevelSoundEvent", "AddItemEntity", "MobEquipment", "MobArmorEquipment", "MoveEntityAbsolute",
    "InventorySlot", "Animate",
];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some((
        match name {
            "parrots" => parrots(),
            "drownedloot" => drowned_loot(),
            "piglinkeep" => piglin_keep(),
            "piglinkeep2" => piglin_kept(),
            _ => return None,
        },
        COMPANION_SHOWN,
    ))
}

/// A parrot fed seeds by the actor in survival until tame; then it is left to fly onto the actor's
/// shoulder, and the actor jumps, sneaks, is hurt and falls, each time waiting for it to come back;
/// last the actor leaves and joins again with it on.
pub fn parrots() -> Vec<String> {
    let mut lines = with(floor(), &["time set noon", "gamemode survival @s", "effect @s resistance 300 4 true", "summon parrot 2.5 -26 0.5", "replaceitem entity @s slot.weapon.mainhand 0 wheat_seeds 64", ""]);
    for _ in 0..8 {
        lines.extend(["!interact parrot".to_owned(), String::new()]);
    }
    waits(&mut lines, 12);
    lines.extend(["!jump", "", "!stop"].map(String::from));
    waits(&mut lines, 12);
    lines.extend(["!sneak", "", "", "!unsneak"].map(String::from));
    waits(&mut lines, 12);
    lines.push("damage @s 1 entity_attack".into());
    waits(&mut lines, 12);
    lines.push("tp @s ~ ~4 ~".into());
    waits(&mut lines, 12);
    lines.extend(["!rejoin", "", "", ""].map(String::from));
    with(lines, &["kill @e[type=parrot]", "gamemode creative @s", ""])
}

/// Twenty drowned armed with tridents killed at once, twice: which of them leave their trident.
pub fn drowned_loot() -> Vec<String> {
    let mut lines = with(floor(), &["time set noon", "fill 3 -27 -6 14 -21 6 water"]);
    for _ in 0..2 {
        lines.extend((0..20).map(|n| format!("summon drowned {}.5 -24 {}.5 0 0 minecraft:as_ranged_adult", 4 + n % 10, n / 10 * 3 - 2)));
        lines.extend(["", "", "damage @e[type=drowned] 100 entity_attack entity @s", "", "", "", "kill @e[type=item]", ""].map(String::from));
    }
    with(lines, &["fill 3 -27 -6 14 -21 6 stone", ""])
}

/// A piglin finds a golden sword thrown near it and keeps it, then a gold ingot it admires; left
/// in the world for `piglinkeep2` after a restart.
pub fn piglin_keep() -> Vec<String> {
    let mut lines = with(floor(), &["summon piglin 3.5 -26 -1.5 0 0 minecraft:spawn_adult", "replaceitem entity @s slot.hotbar 0 golden_sword 1", "", "!drop 0 1"]);
    waits(&mut lines, 12);
    lines.extend(["replaceitem entity @s slot.hotbar 0 gold_ingot 1", "", "!drop 0 1", "", "", ""].map(String::from));
    lines
}

/// After `piglinkeep`'s restart (KEEP_WORLD=1): the actor joins again and is shown the piglin.
pub fn piglin_kept() -> Vec<String> {
    let mut lines: Vec<String> = ["!rejoin", "", ""].map(String::from).into();
    waits(&mut lines, 10);
    lines
}
