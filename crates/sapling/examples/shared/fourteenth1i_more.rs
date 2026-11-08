//! fourteenth1i_drill.rs's other scenes. `i1misc`: a slab put under a top slab, of its kind and another,
//! and on a bottom slab's top; bone meal on a hanging mangrove propagule (stage 0, then 4) and eight
//! hanging ones under random ticks; a torch clock burnt out, its dust broken soon after and again long
//! after; twelve spawners and each sculk block broken by `/fill ... destroy`. `i1anvil`: anvils dropped on
//! the survival actor in an iron helmet, on held cows, and two rows dropped 11 and 3 twice over.
//! `i1trip`: a tripwire line broken by hand and by shears, a gap filled by hand, a hook put by hand.
//! `i1chip`: the anvil rows alone, four times, for how often a landing chips one.

use crate::creatures::{floor, waits, with};

const MISC: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "LevelEvent", "LevelSoundEvent", "AddItemEntity", "AddEntity", "InventorySlot", "CommandOutput"];
const ANVIL: &[&str] = &[
    "UpdateBlock", "UpdateBlockSynced", "LevelEvent", "LevelSoundEvent", "AddItemEntity", "AddEntity", "RemoveEntity", "EntityEvent", "UpdateAttributes", "InventorySlot", "InventoryContent", "MobArmorEquipment",
    "CommandOutput",
];
const TRIP: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "LevelSoundEvent", "LevelEvent", "AddItemEntity", "InventorySlot", "CommandOutput"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "i1misc" => (misc(), MISC),
        "i1anvil" => (anvil(), ANVIL),
        "i1trip" => (trip(), TRIP),
        "i1chip" => (chip(), &["UpdateSubchunkBlocks", "UpdateBlockSynced", "CommandOutput"]),
        _ => return None,
    })
}

fn hold(item: &str) -> String {
    format!("replaceitem entity @s slot.weapon.mainhand 0 {item}")
}

const TOP_SLAB: &str = "smooth_stone_slab [\"minecraft:vertical_half\"=\"top\"]";

fn misc() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher 14.5 -26 7.5", "gamerule randomtickspeed 0"]);
    // Slabs: under a top slab with air below it, of its kind and of another; on a bottom slab.
    for item in ["smooth_stone_slab 8", "stone_brick_slab 8"] {
        lines.extend([format!("setblock 2 -24 0 {TOP_SLAB}"), hold(item), "!click 2 2 0 down".into(), String::new(), "fill 2 -26 0 2 -24 0 air".into()]);
    }
    lines.extend([format!("setblock 2 -25 0 {TOP_SLAB}"), hold("smooth_stone_slab 8"), "!click 2 1 0 up".into(), String::new(), "fill 2 -26 0 2 -23 0 air".into()]);
    lines.extend(["setblock 2 -26 0 smooth_stone_slab".into(), "!click 2 0 0 up".into(), "!click 2 0 0 up".into(), String::new(), "fill 2 -26 0 2 -23 0 air".into()]);
    // Bone meal on a hanging propagule, stage 0 and then 4.
    for stage in [0, 4] {
        lines.extend([
            "setblock 2 -24 0 mangrove_leaves [\"persistent_bit\"=true]".into(),
            format!("setblock 2 -25 0 mangrove_propagule [\"hanging\"=true,\"propagule_stage\"={stage}]"),
            hold("bone_meal 16"),
        ]);
        lines.extend(std::iter::repeat_n("!click 2 1 0 down".to_string(), if stage == 0 { 5 } else { 2 }));
        lines.extend([String::new(), "fill 2 -25 0 2 -24 0 air".into(), "kill @e[type=item]".into()]);
    }
    // Random ticks on eight hanging propagules.
    lines.extend(["fill 4 -23 -4 11 -23 -4 mangrove_leaves [\"persistent_bit\"=true]".into(), "fill 4 -24 -4 11 -24 -4 mangrove_propagule [\"hanging\"=true,\"propagule_stage\"=0]".into(), "gamerule randomtickspeed 100".into()]);
    waits(&mut lines, 15);
    lines.extend(["gamerule randomtickspeed 1".into(), "fill 4 -24 -4 11 -23 -4 air".into(), "clear @s".into()]);
    // A torch on a stone's east face, stone over it, dust and a lamp on the first: burnt out, its dust
    // broken 2 s after; put back and burnt out again, broken 10 s after.
    lines.extend(["setblock 6 -26 4 stone", "setblock 7 -25 4 stone", "setblock 5 -25 4 redstone_lamp", "setblock 6 -25 4 redstone_wire", "setblock 7 -26 4 redstone_torch [\"torch_facing_direction\"=\"west\"]"].map(String::from));
    waits(&mut lines, 4);
    lines.push("setblock 6 -25 4 air".into());
    waits(&mut lines, 4);
    lines.push("setblock 6 -25 4 redstone_wire".into());
    waits(&mut lines, 12);
    lines.push("setblock 6 -25 4 air".into());
    waits(&mut lines, 4);
    lines.push("fill 5 -26 4 7 -25 4 air".into());
    // Twelve of each broken by `/fill ... destroy`, both players far off.
    lines.extend(["tp @s 15.5 -26 -7.5".into(), "tp CmdWatcher 15.5 -26 7.5".into()]);
    for (row, block) in XP_BLOCKS.iter().enumerate() {
        let z = -6 + 2 * row as i32;
        lines.extend((0..12).map(|i| format!("setblock {} -26 {z} {block}", -8 + 2 * i)));
        lines.extend([format!("fill -8 -26 {z} 14 -26 {z} air destroy"), String::new(), "kill @e[type=item]".into(), "kill @e[type=xp_orb]".into()]);
    }
    with(lines, &["tp @s 0.5 -26 0.5 -90 0", ""])
}

const XP_BLOCKS: [&str; 7] = ["mob_spawner", "sculk", "sculk_vein", "sculk_sensor", "calibrated_sculk_sensor", "sculk_catalyst", "sculk_shrieker"];

const HEAL: &str = "effect @s instant_health 1 4";

fn anvil() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher 14.5 -26 7.5", "gamemode survival @s", "replaceitem entity @s slot.armor.head 0 iron_helmet", ""]);
    // On the helmeted actor from 7 and 3 over its feet, then bare from 3.
    for (over, helmet) in [(7, true), (3, true), (3, false)] {
        if !helmet {
            lines.push("replaceitem entity @s slot.armor.head 0 air".into());
        }
        lines.extend(["tp @s 0.5 -26 0.5 -90 0".into(), format!("setblock 0 {} 0 anvil", -26 + over), String::new(), String::new(), "tp @s 0.5 -26 3.5 -90 0".into(), "setblock 0 -26 0 air".into(), HEAL.into(), "kill @e[type=item]".into(), String::new()]);
    }
    lines.extend(["gamemode creative @s", "tp @s 0.5 -26 -4.5 -90 0"].map(String::from));
    // Held cows: one under a drop of 4, one under 16.
    for (x, over) in [(4, 4), (8, 16)] {
        lines.extend([format!("summon cow {x}.5 -26 0.5 0 0 minecraft:entity_spawned"), "effect @e[type=cow] slowness 100 255 true".into(), format!("setblock {x} {} 0 anvil", -26 + over)]);
    }
    waits(&mut lines, 3);
    lines.extend(["kill @e[type=cow]", "fill -8 -26 -1 16 -26 1 air", "kill @e[type=item]", ""].map(String::from));
    // Two rows of twenty-three: one falling 11, one falling 3, twice.
    for _ in 0..2 {
        lines.extend(["fill -7 -15 -6 15 -15 -6 anvil", "fill -7 -23 -3 15 -23 -3 anvil"].map(String::from));
        waits(&mut lines, 4);
        lines.extend(["fill -8 -26 -7 16 -16 -2 air", "kill @e[type=item]", ""].map(String::from));
    }
    lines
}

/// Rows of twenty-three anvils falling 11 and 3, four times, for how often a landing chips one.
fn chip() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher 14.5 -26 7.5", "tp @s 0.5 -26 4.5 -90 0"]);
    for _ in 0..4 {
        lines.extend(["fill -7 -15 -6 15 -15 -6 anvil", "fill -7 -23 -3 15 -23 -3 anvil"].map(String::from));
        waits(&mut lines, 4);
        lines.extend(["fill -8 -26 -7 16 -16 -2 air", "kill @e[type=item]", ""].map(String::from));
    }
    lines
}

fn trip() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher 14.5 -26 7.5", "gamemode survival @s", "tp @s 0.5 -26 0.5 -90 0"]);
    let line = ["fill -4 -26 3 4 -26 3 air", "setblock -4 -26 3 stone", "setblock 4 -26 3 stone", "setblock -3 -26 3 tripwire_hook [\"direction\"=3]", "setblock 3 -26 3 tripwire_hook [\"direction\"=1]", "fill -2 -26 3 2 -26 3 trip_wire", ""];
    // By hand, then by shears.
    for item in ["air", "shears"] {
        lines = with(lines, &line);
        lines.extend([hold(item), "!break 0 0 3".into(), String::new(), String::new(), "kill @e[type=item]".into()]);
    }
    // A gap in the wire filled by hand.
    lines = with(lines, &line);
    lines.extend(["setblock 0 -26 3 air".into(), String::new(), hold("string 4"), "!place 0 -1 3 up".into(), String::new()]);
    // The east hook taken away, then put back by hand.
    lines.extend(["setblock 3 -26 3 air".into(), String::new(), "kill @e[type=item]".into(), hold("tripwire_hook 2"), "!place 4 0 3 west".into(), String::new(), String::new()]);
    with(lines, &["gamemode creative @s", "fill -4 -26 3 4 -26 3 air", "kill @e[type=item]", "clear @s", ""])
}
