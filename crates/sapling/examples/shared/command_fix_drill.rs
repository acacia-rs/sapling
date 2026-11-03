//! Drills of the commands follow-up (research/parity/FIX-BRIEF.md "Group 1"): what `/setblock` writes
//! (beds, coral fans, kelp and vines), comparators beside a container set by command, `/time set`
//! names, `/fill` in the Nether, `/damage ... entity <who>`, `/replaceitem` banners, mounts and mobs.
//! On creature_drill.rs's floor (stone at y -27).

use super::creatures::{floor, waits, with};

/// Packets shown of the block drill: blocks named (UpdateSubchunkBlocks turns naming on).
pub const BLOCKS_SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "LevelEvent"];
/// Packets shown of the other drills.
pub const MISC_SHOWN: &[&str] = &["EntityEvent", "ActorEvent", "ChangeDimension", "Respawn", "MobEquipment", "MobArmorEquipment", "SetEntityData", "InventorySlot", "LevelSoundEvent", "AddItemEntity", "UpdateEquipment"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "fixset" => (set_blocks(), BLOCKS_SHOWN),
        "fixmisc" => (misc(), MISC_SHOWN),
        "fixreplace" => (replace_items(), MISC_SHOWN),
        _ => return None,
    })
}

const COMPARATOR: &str = "unpowered_comparator [\"minecraft:cardinal_direction\"=\"west\",\"output_subtract_bit\"=false]";

/// Beds, coral fans and a wall fan, kelp in a glass trough of water, twisting vines on the floor,
/// weeping and cave vines under stone, watched 6 s; then a comparator reading a chest that commands
/// fill and replace (barrel, cake, air), and one set beside a chest already full.
pub fn set_blocks() -> Vec<String> {
    let mut lines = with(floor(), &[
        "setblock 10 -26 -4 bed",
        "setblock 12 -26 -6 bed [\"direction\"=1,\"head_piece_bit\"=true]",
        "setblock 4 -26 2 tube_coral_fan",
        "setblock 5 -26 2 brain_coral_fan [\"coral_fan_direction\"=1]",
        "setblock 6 -26 1 stone",
        "setblock 6 -26 2 tube_coral_wall_fan [\"coral_direction\"=3]",
        "fill 3 -27 4 13 -25 6 glass",
        "fill 4 -26 5 12 -26 5 water",
    ]);
    lines.extend((4..=7).map(|x| format!("setblock {x} -26 5 kelp")));
    lines.extend((8..=10).map(|x| format!("setblock {x} -26 -2 twisting_vines")));
    lines.push("fill 8 -24 0 13 -24 0 stone".into());
    lines.extend((8..=10).map(|x| format!("setblock {x} -25 0 weeping_vines")));
    lines.extend((11..=13).map(|x| format!("setblock {x} -25 0 cave_vines")));
    waits(&mut lines, 6);
    lines.extend(["setblock 0 -26 -4 chest", &format!("setblock 1 -26 -4 {COMPARATOR}"), "setblock 2 -26 -4 redstone_wire", ""].map(String::from));
    lines.extend(["replaceitem block 0 -26 -4 slot.container 0 stone 64", "", "setblock 0 -26 -4 barrel", "", "setblock 0 -26 -4 cake", "", "setblock 0 -26 -4 air", ""].map(String::from));
    lines.extend(["setblock 0 -26 -6 chest", "replaceitem block 0 -26 -6 slot.container 0 stone 64", "", &format!("setblock 1 -26 -6 {COMPARATOR}"), "setblock 2 -26 -6 redstone_wire", "", ""].map(String::from));
    lines
}

/// `/time set` names and queries; `/fill` relative in the overworld and absolute and relative in the
/// Nether; `/damage` with a damager, at a zombie and then at the actor until it dies.
pub fn misc() -> Vec<String> {
    let mut lines = with(floor(), &[
        "time query daytime",
        "time query gametime",
        "time set noon",
        "time query daytime",
        "time set noon",
        "time set midnight",
        "time set noon",
        "time set 100",
        "time query daytime",
        "time set noon",
        "fill ~1 ~-1 ~1 ~2 ~-1 ~2 dirt",
        "fill ~1 ~ ~1 ~2 ~ ~2 dirt",
        "execute in nether run tp @s 0.5 100 0.5",
    ]);
    waits(&mut lines, 3);
    lines.extend(["fill -3 99 -3 3 99 3 stone", "fill ~-1 ~-1 ~-1 ~1 ~-1 ~1 netherrack", "setblock 0 98 0 glowstone", "testforblock 0 99 0 netherrack", "execute in overworld run tp @s 0.5 -26 0.5"].map(String::from));
    waits(&mut lines, 3);
    lines.extend([
        "summon zombie 4.5 -26 0.5",
        "effect @e[type=zombie] slowness 200 255 true",
        "",
        "damage @e[type=zombie] 2 entity_attack entity @s",
        "damage @e[type=zombie] 2 entity_attack entity CmdWatcher",
        "damage @e[type=zombie] 2 fire entity @s",
        "damage @e[type=zombie] 2 entity @s",
        "damage @e[type=zombie] 2 entity_attack entity @e[type=cow]",
        "damage @e[type=zombie] 2 entity_attack entity @a",
        "damage @e[type=zombie] 2 entity_attack entity",
        "gamemode survival @s",
        "damage @s 1 entity_attack entity @e[type=zombie]",
        "",
        "damage @s 100 projectile entity @e[type=zombie]",
    ].map(String::from));
    waits(&mut lines, 3);
    with(lines, &["gamemode creative @s", "tp @s 0.5 -26 0.5", "kill @e[type=!player]", ""])
}

/// Banners put in the hand by `/replaceitem` (plain and aux 15) and given; then tamed mounts and a
/// zombie given tack, armour, chests and goods into each slot type; the donkey killed.
pub fn replace_items() -> Vec<String> {
    let mut lines = with(floor(), &["replaceitem entity @s slot.hotbar 0 banner", "replaceitem entity @s slot.hotbar 1 banner 1 15", "give @s banner 1 15", "", "clear @s"]);
    for kind in ["horse", "donkey", "llama"] {
        let z = match kind { "horse" => 3, "donkey" => -3, _ => 6 };
        lines.extend([format!("summon {kind} 4.5 -26 {z}.5 0 0 minecraft:spawn_tame_adult"), format!("effect @e[type={kind}] slowness 200 255 true"), String::new()]);
        for (slot, item) in [("slot.saddle", "saddle"), ("slot.armor", "iron_horse_armor"), ("slot.armor", "white_carpet"), ("slot.chest", "chest"), ("slot.inventory", "diamond 5"), ("slot.inventory", "emerald 2")] {
            lines.push(format!("replaceitem entity @e[type={kind}] {slot} 0 {item}"));
        }
        lines.push(String::new());
    }
    lines.extend(["summon zombie 8.5 -26 0.5", "effect @e[type=zombie] slowness 200 255 true", ""].map(String::from));
    for (slot, item) in [("slot.armor.head", "iron_helmet"), ("slot.armor.chest", "iron_chestplate"), ("slot.weapon.mainhand", "iron_sword"), ("slot.weapon.offhand", "shield"), ("slot.saddle", "saddle"), ("slot.inventory", "stone"), ("slot.hotbar", "stone"), ("slot.armor.feet", "stone")] {
        lines.push(format!("replaceitem entity @e[type=zombie] {slot} 0 {item}"));
    }
    lines.extend(["", "kill @e[type=donkey]", "", "", "kill @e[type=!player]", ""].map(String::from));
    lines
}
