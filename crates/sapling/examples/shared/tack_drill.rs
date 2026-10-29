//! Drills of mounts' tack and chests (docs/research/mob-life.md "Mount inventories", leftovers.md
//! "Mounts"): llama strength, carpets, the horse screen, what a dying donkey or llama spills, bred
//! llamas. On eggs_drill.rs's walled floor.

use super::creatures::{waits, with};
use super::eggs::penned;

/// Packets shown of these drills.
pub const TACK_SHOWN: &[&str] = &[
    "AddEntity", "RemoveEntity", "SetEntityData", "EntityEvent", "ActorEvent", "LevelSoundEvent", "AddItemEntity", "MobEquipment", "MobArmorEquipment", "ContainerOpen", "ContainerClose",
    "InventoryContent", "InventorySlot", "UpdateEquipment", "ItemStackResponse", "SetEntityLink",
];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some((
        match name {
            "tack" => tack(),
            "llamabreed" => llama_breed(),
            _ => return None,
        },
        TACK_SHOWN,
    ))
}

/// `!feedall kind`: uses the held item on every mob of that kind it knows of, oldest first;
/// `!openinv`: the inventory key (`Interact` OpenInventory). False for any other line.
pub async fn act(actor: &mut acacia_bot::Bot, line: &str) -> bool {
    if let Some(kind) = line.strip_prefix("!feedall ") {
        let kind = format!("minecraft:{kind}");
        let mut ids: Vec<u64> = actor.state().entities.iter().filter(|entity| entity.kind == kind).map(|entity| entity.runtime_id).collect();
        ids.sort_unstable();
        for id in ids {
            println!("## fed {id}: {:?}", actor.interact_entity(id).await);
        }
        return true;
    }
    if line == "!openinv" {
        actor.open_inventory();
        return true;
    }
    false
}

/// A tamed grown `kind` held still two blocks east of the actor's spot.
fn tamed(kind: &str) -> Vec<String> {
    vec![format!("summon {kind} 2.5 -26 0.5 0 0 minecraft:spawn_tame_adult"), format!("effect @e[type={kind}] slowness 200 255 true"), String::new()]
}

/// The actor, put back at its spot, clicks the newest `kind` holding `item` (sneaking if `sneak`).
fn click(kind: &str, item: &str, sneak: bool) -> Vec<String> {
    let mut lines = vec!["clear @s".to_owned(), format!("give @s {item}"), "tp @s 0.5 -26 0.5 -90 0".into(), String::new()];
    lines.extend(sneak.then(|| "!sneak".to_owned()));
    lines.extend([format!("!interact {kind}"), String::new()]);
    lines.extend(sneak.then(|| "!unsneak".to_owned()));
    lines
}

/// A tamed donkey or llama given a chest by a sneaking click, then a saddle and goods by command.
fn chested(kind: &str) -> Vec<String> {
    let mut lines = tamed(kind);
    lines.extend(click(kind, "chest", true));
    lines.extend([format!("replaceitem entity @e[type={kind}] slot.saddle 0 saddle"), format!("replaceitem entity @e[type={kind}] slot.inventory 0 diamond 5"), format!("replaceitem entity @e[type={kind}] slot.inventory 3 emerald 2"), String::new()]);
    lines
}

/// Sixteen llamas summoned (their strength); a tamed llama given a carpet and a chest by clicks,
/// opened sneaking; a tamed horse with saddle and armour opened sneaking and from its back; a
/// chested donkey with a saddle and goods killed with `doMobLoot` on and off, the llama and horse
/// killed; then a chested donkey and a carpeted llama left in the world for its folder to be read.
pub fn tack() -> Vec<String> {
    let mut lines = penned(&["time set noon"]);
    lines.extend((0..16).map(|n| format!("summon llama {}.5 -26 -6.5", n - 6)));
    waits(&mut lines, 2);
    lines.push("kill @e[type=llama]".into());
    lines.extend(tamed("llama"));
    lines.extend(click("llama", "white_carpet", false));
    lines.extend(click("llama", "chest", true));
    lines.extend(["replaceitem entity @e[type=llama] slot.inventory 0 diamond 3".to_owned(), String::new()]);
    lines.extend(click("llama", "stick", true));
    lines.extend(["!close", "", "kill @e[type=llama]", "", ""].map(String::from));
    lines.extend(tamed("horse"));
    lines.extend(["replaceitem entity @e[type=horse] slot.saddle 0 saddle", "replaceitem entity @e[type=horse] slot.armor.body 0 iron_horse_armor", ""].map(String::from));
    lines.extend(click("horse", "stick", true));
    lines.extend(["!close", "", "!mount horse", "", "!openinv", "", "", "!close", "!dismount", "", "kill @e[type=horse]", "", ""].map(String::from));
    lines.extend(chested("donkey"));
    lines.extend(["kill @e[type=donkey]", "", "", "kill @e[type=item]", "gamerule domobloot false"].map(String::from));
    lines.extend(chested("donkey"));
    lines.extend(["kill @e[type=donkey]", "", "", "gamerule domobloot true", "kill @e[type=item]"].map(String::from));
    lines.extend(chested("donkey"));
    lines.extend(["tp @e[type=donkey] 4.5 -26 4.5".to_owned()]);
    lines.extend(tamed("llama"));
    lines.extend(click("llama", "white_carpet", false));
    with(lines, &["clear @s"])
}

/// Ten tamed llamas in pairs about the actor, each fed a hay bale: the babies' strength against
/// their parents'.
pub fn llama_breed() -> Vec<String> {
    let mut lines = penned(&["time set noon", "give @s hay_block 64"]);
    for (x, z) in [(2, 0), (2, 1), (-1, 2), (0, 2), (0, -2), (1, -2), (-2, 0), (-2, -1), (2, -2), (2, 2)] {
        lines.push(format!("summon llama {x}.5 -26 {z}.5 0 0 minecraft:spawn_tame_adult"));
    }
    lines.extend(["", "!feedall llama"].map(String::from));
    waits(&mut lines, 20);
    lines
}
