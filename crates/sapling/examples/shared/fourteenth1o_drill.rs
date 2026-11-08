//! The fourteenth wave's group O drills (research/parity/FOURTEENTH-BRIEF.md), on creature_drill.rs's floor
//! (stone at y -27, the actor at 0.5 -26 0.5): a survival actor hits a zombie pigman with two more 7 off, then
//! `/damage`s one so (`o1pig`); a forced creeper bursts under a falling slab of sand, mob griefing off
//! (`o1blast`); the actor tames a wolf and smashes a golem with a mace, the wolf sat 2.2 off and a wild one as
//! far the other side (`o1pets`); piglins come to a dropped golden helmet, cooked porkchop and crossbow and are
//! killed (`o1piglin`); the actor dies at 1 health under a falling stalactite, twice, and by a starred rocket
//! under a glass roof (`o1fire`).

use super::creatures::{floor, waits, with};

const PIG_SHOWN: &[&str] = &["SetEntityData", "EntityEvent", "LevelSoundEvent", "SetEntityMotion", "UpdateAttributes", "CommandOutput", "AddEntity", "Animate"];
const BLAST_SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "SetEntityMotion", "MoveEntityDelta", "LevelEvent", "LevelSoundEvent", "UpdateBlock", "CommandOutput", "SetEntityData"];
const PETS_SHOWN: &[&str] = &["EntityEvent", "Animate", "LevelSoundEvent", "SetEntityMotion", "SetEntityData", "CommandOutput", "AddEntity", "UpdateAttributes"];
const PIGLIN_SHOWN: &[&str] = &[
    "AddEntity", "AddItemEntity", "RemoveEntity", "TakeItemEntity", "MobEquipment", "MobArmorEquipment", "SetEntityData", "LevelSoundEvent", "EntityEvent", "CommandOutput",
];
const FIRE_SHOWN: &[&str] = &[
    "Text", "DeathInfo", "EntityEvent", "SetHealth", "Respawn", "UpdateAttributes", "LevelSoundEvent", "AddEntity", "RemoveEntity", "CommandOutput", "ItemStackResponse", "UpdateBlock",
    "AddItemEntity", "InventoryContent", "InventorySlot", "CompletedUsingItem",
];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "o1pig" => (pigmen(), PIG_SHOWN),
        "o1blast" => (blast(), BLAST_SHOWN),
        "o1pets" => (pets(), PETS_SHOWN),
        "o1piglin" => (piglins(), PIGLIN_SHOWN),
        "o1fire" => (deaths(), FIRE_SHOWN),
        _ => return None,
    })
}

fn hold(item: &str) -> String {
    format!("replaceitem entity @s slot.weapon.mainhand 0 {item}")
}

const HEAL: &str = "effect @s instant_health 1 4 true";
const PIGMEN: [&str; 4] = ["summon zombie_pigman -4.5 -26 -4.5", "summon zombie_pigman -4.5 -26 4.5", "summon zombie_pigman 2.5 -26 0.5", "effect @e[type=zombie_pigman] slowness 200 255 true"];

/// A melee hit on the pigman 2 east of the warded survival actor, two more 7 off west; then the same by `/damage`.
fn pigmen() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher 0.5 -26 7.5", "gamemode survival @s", "effect @s resistance 200 4 true", "!look 2 1 0"]);
    lines = with(lines, &PIGMEN);
    lines = with(lines, &["", "!hit zombie_pigman"]);
    waits(&mut lines, 3);
    lines = with(lines, &["kill @e[type=zombie_pigman]", "tp @s 0.5 -26 0.5 -90 0"]);
    lines = with(lines, &PIGMEN);
    lines = with(lines, &["", "damage @e[type=zombie_pigman,c=1] 1 entity_attack entity @s"]);
    waits(&mut lines, 3);
    with(lines, &["kill @e[type=!player]", "gamemode creative @s", "effect @s clear", ""])
}

/// A creeper forced to burst at 4.5 -26 0.5 and, a second on, a two-high slab of sand set 3 over the floor
/// east of it (x 5..9, z -3..3), falling as it bursts; mob griefing off so no block breaks.
fn blast() -> Vec<String> {
    let mut lines = with(floor(), &["gamerule mobgriefing false", "tp CmdWatcher 0.5 -26 -7.5", "tp @s -7.5 -26 0.5 -90 0", "", "summon creeper 4.5 -26 0.5 0 0 minecraft:start_exploding_forced", "fill 5 -23 -3 9 -22 3 sand"]);
    waits(&mut lines, 4);
    with(lines, &["kill @e[type=!player]", "fill -8 -26 -8 16 -16 8 air", "gamerule mobgriefing true", "tp @s 0.5 -26 0.5 -90 0", ""])
}

/// Tames a held wolf with bones (as many tries as it takes) and sits it with the mace; a wild wolf 2.2 off the
/// golem's other side; the survival actor smashes the golem from 4 up.
fn pets() -> Vec<String> {
    let pen = ["fill 1 -26 -1 3 -26 1 barrier", "fill 2 -26 0 2 -26 0 air"];
    let mut lines = with(floor(), &["gamerule pvp true", "gamemode survival @s", "tp CmdWatcher 0.5 -26 7.5", "effect @s resistance 200 4 true"]);
    lines = with(lines, &["summon wolf 1.5 -26 -1.5", "effect @e[type=wolf] slowness 200 255 true", &hold("bone 32"), "!look 1 0.5 -2", ""]);
    for _ in 0..16 {
        lines = with(lines, &["!interact wolf"]);
    }
    lines = with(lines, &[&hold("mace"), "", "!interact wolf", ""]);
    lines = with(lines, &pen);
    lines = with(lines, &["summon wolf 3.5 -26 2.5", "effect @e[type=wolf] slowness 200 255 true", "summon iron_golem 2.5 -26 0.5", HEAL, "", "!smash iron_golem 4 2.2"]);
    waits(&mut lines, 3);
    with(lines, &["kill @e[type=!player]", "gamemode creative @s", "effect @s clear", "clear @s", "fill -8 -26 -8 16 -16 8 air", ""])
}

/// For each item, a piglin 3 east of the creative actor and the item dropped towards it; killed 6 s on.
fn piglins() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher 0.5 -26 7.5"]);
    for item in ["golden_helmet", "cooked_porkchop", "crossbow", "golden_sword"] {
        lines = with(lines, &["kill @e[type=item]", "summon piglin 3.5 -26 0.5 90 0 minecraft:spawn_adult", &format!("replaceitem entity @s slot.hotbar 0 {item} 1"), "", "!drop 0 1"]);
        waits(&mut lines, 6);
        lines = with(lines, &["kill @e[type=piglin]", "", ""]);
    }
    with(lines, &["kill @e[type=!player]", "clear @s", ""])
}

/// Low to 1 health under a stalactite set falling, twice; then a starred rocket crafted in a table and launched
/// under a glass roof 3 over the floor. A creative meal after the craft makes BDS send the whole inventory, so
/// the bot holds the rocket it crafted.
fn deaths() -> Vec<String> {
    let low = |lines: Vec<String>, at: &str| with(lines, &[&format!("tp @s {at}"), "gamemode survival @s", HEAL, "", "damage @s 19", ""]);
    let mut lines = with(floor(), &["tp CmdWatcher 0.5 -26 7.5", "gamerule showdeathmessages true", "gamerule doimmediaterespawn true", ""]);
    for _ in 0..2 {
        lines = with(low(lines, "12.5 -26 -3.5"), &["setblock 12 -17 -4 stone", "setblock 12 -18 -4 pointed_dripstone [\"hanging\"=true,\"dripstone_thickness\"=\"tip\"]", "", "setblock 12 -17 -4 air", "", "", "kill @e[type=item]"]);
    }
    lines = with(lines, &[
        "gamemode creative @s", "tp @s 0.5 -26 0.5 0 0", "setblock 0 -26 2 crafting_table", "clear @s", "replaceitem entity @s slot.hotbar 1 gunpowder 2", "replaceitem entity @s slot.hotbar 2 red_dye 1",
        "replaceitem entity @s slot.hotbar 3 paper 1", "", "!click 0 0 2", "", "!grid! fireworks 1 1 2", "", "!grid! fireworks 3 3 1 0", "", "!close", "",
        "replaceitem entity @s slot.hotbar 5 bread 1", "!hotbar 5", "", "!eat",
    ]);
    waits(&mut lines, 3);
    lines = with(lines, &["!hotbar 0", "", "fill -3 -23 -3 3 -23 -1 glass", "tp @s 0.5 -26 -1.5 0 0"]);
    lines = with(low(lines, "0.5 -26 -1.5 0 0"), &["!look 0 0 1.5", "", "!click 0 -1 1"]);
    waits(&mut lines, 4);
    with(lines, &["kill @e[type=!player]", "gamemode creative @s", "clear @s", "tp @s 0.5 -26 0.5 -90 0", "fill -8 -26 -8 16 -16 8 air", "fill -8 -27 -8 16 -27 8 stone", ""])
}
