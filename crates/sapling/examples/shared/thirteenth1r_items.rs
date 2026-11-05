//! Group R's item drills: shots (`r1shots`, `r1stuck`), dispensers (`r1disp`), a bundle given to
//! an allay (`r1bundle`), anvil merges (`r1anvil`), armour Unbreaking (`r1wear`), buckets
//! (`r1buckets`) and fishing (`r1fish`).

use crate::creatures::{floor, waits, with};

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "r1shots" => (shots(), SHOTS_SHOWN),
        "r1stuck" => (stuck(), SHOTS_SHOWN),
        "r1disp" => (dispensed(), SHOTS_SHOWN),
        "r1bundle" => (bundle(), BUNDLE_SHOWN),
        "r1anvil" => (anvil(), ANVIL_SHOWN),
        "r1wear" => (wear(), WEAR_SHOWN),
        "r1buckets" => (buckets(), BUCKETS_SHOWN),
        "r1fish" => (fish(), FISH_SHOWN),
        _ => return None,
    })
}

fn hold(item: &str) -> String {
    format!("replaceitem entity @s slot.weapon.mainhand 0 {item}")
}

fn owned<const N: usize>(lines: [&str; N]) -> impl Iterator<Item = String> {
    lines.into_iter().map(String::from)
}

const SHOTS_SHOWN: &[&str] = &["AddEntity", "AddItemEntity", "SetEntityMotion", "MoveEntityDelta", "MoveEntityAbsolute", "RemoveEntity", "LevelSoundEvent", "LevelEvent", "InventorySlot", "SetEntityData", "EntityEvent", "TakeItemEntity", "MovePlayer", "CorrectPlayerMovePrediction"];

/// In survival, facing east: a bow loosing a poison arrow; a Multishot crossbow loaded then shot; a
/// Loyalty III trident thrown at the floor; a Riptide III trident let go standing in water.
fn shots() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher 4.5 -26 6.5", "gamemode survival @s", "tp @s 0.5 -26 0.5 -90 0", "!look 8 1.6 0", ""]);
    lines.extend([hold("bow"), "replaceitem entity @s slot.hotbar 1 arrow 4 26".into()]);
    lines.extend(owned(["", "!use", "", "!release", "", "", "kill @e[type=arrow]", "clear @s"]));
    lines.extend([hold("crossbow"), "enchant @s multishot 1".into(), "replaceitem entity @s slot.hotbar 1 arrow 8".into()]);
    lines.extend(owned(["", "!use", "", "", "!release", "", "!use", "", "", "kill @e[type=arrow]", "clear @s"]));
    lines.extend([hold("trident"), "enchant @s loyalty 3".into()]);
    lines.extend(owned(["!look 5 0 0", "", "!use", "", "!release"]));
    waits(&mut lines, 8);
    lines.extend(owned(["kill @e[type=thrown_trident]", "clear @s", "fill -6 -26 -6 -3 -25 -3 water", "tp @s -4.5 -26 -4.5 -90 0", "", ""]));
    lines.extend([hold("trident"), "enchant @s riptide 3".into()]);
    lines.extend(owned(["!look 8 3 0", "", "!use", "", "!release", "", "", ""]));
    with(lines, &["fill -6 -26 -6 -3 -25 -3 air", "gamemode creative @s", "clear @s", "tp CmdWatcher 0.5 -26 0.5", "kill @e[type=!player]", ""])
}

/// An arrow shot into a wall in creative and one in survival, then a minute and more of waiting.
fn stuck() -> Vec<String> {
    let mut lines = with(floor(), &["fill 6 -26 -2 6 -23 2 stone", "tp @s 0.5 -26 0.5 -90 0", "!look 8 1.6 0", ""]);
    for mode in ["creative", "survival"] {
        lines.extend([format!("gamemode {mode} @s"), hold("bow"), "replaceitem entity @s slot.hotbar 1 arrow 4".into()]);
        lines.extend(owned(["", "!use", "", "!release", "", "tp @s 0.5 -26 -1.5 -90 0", "!look 8 1.6 0", ""]));
    }
    waits(&mut lines, 66);
    with(lines, &["fill 6 -26 -2 6 -23 2 air", "gamemode creative @s", "clear @s", "kill @e[type=!player]", ""])
}

/// `setblock` of a redstone block under (or over) a dispenser and back.
fn pulse(lines: &mut Vec<String>, x: i32, y: i32, z: i32) {
    lines.extend([format!("setblock {x} {y} {z} redstone_block"), String::new(), format!("setblock {x} {y} {z} stone"), String::new()]);
}

/// A dispenser facing south shooting twelve arrows at a wall, a survival actor walking along the
/// wall, then a wind charge; cow eggs dispensed up, down and east; shears at a sheep; dirt.
fn dispensed() -> Vec<String> {
    let mut lines = with(floor(), &["fill -8 -26 8 16 -20 8 stone", "setblock 0 -26 -6 dispenser [\"facing_direction\"=3]", "replaceitem block 0 -26 -6 slot.container 0 arrow 12", "tp @s -5.5 -26 0.5", ""]);
    for _ in 0..12 {
        pulse(&mut lines, 0, -27, -6);
    }
    lines.extend(owned(["gamemode survival @s", "tp @s -2.5 -26 7.5", "", "tp @s -0.5 -26 7.5", "", "tp @s 1.5 -26 7.5", "", "tp @s 3.5 -26 7.5", "", "gamemode creative @s", "tp @s -5.5 -26 0.5"]));
    lines.push("replaceitem block 0 -26 -6 slot.container 0 wind_charge 1".into());
    pulse(&mut lines, 0, -27, -6);
    lines.extend(owned(["", "setblock 4 -26 -6 dispenser [\"facing_direction\"=1]", "setblock 6 -22 -6 dispenser [\"facing_direction\"=0]", "setblock 8 -26 -6 dispenser [\"facing_direction\"=5]"]));
    for (x, y) in [(4, -26), (6, -22), (8, -26)] {
        lines.push(format!("replaceitem block {x} {y} -6 slot.container 0 cow_spawn_egg 1"));
    }
    pulse(&mut lines, 4, -27, -6);
    pulse(&mut lines, 6, -21, -6);
    pulse(&mut lines, 8, -27, -6);
    lines.extend(owned(["kill @e[type=cow]", "setblock 11 -26 -6 dispenser [\"facing_direction\"=3]", "replaceitem block 11 -26 -6 slot.container 0 shears 1", "summon sheep 11.5 -26 -4.5", "effect @e[type=sheep] slowness 999 255 true", ""]));
    pulse(&mut lines, 11, -27, -6);
    lines.extend(owned(["setblock 13 -26 -6 dispenser [\"facing_direction\"=5]", "replaceitem block 13 -26 -6 slot.container 0 dirt 1"]));
    pulse(&mut lines, 13, -27, -6);
    with(lines, &["", "fill -8 -26 -8 16 -16 8 air", "kill @e[type=!player]", "tp @s 0.5 -26 0.5", ""])
}

const BUNDLE_SHOWN: &[&str] = &["ContainerRegistryCleanup", "MobEquipment", "InventorySlot", "ItemStackResponse", "SetEntityData"];

/// In survival a bundle holding dirt given to an allay; forty seconds for the cleanups.
fn bundle() -> Vec<String> {
    let mut lines = with(floor(), &["gamemode survival @s", "replaceitem entity @s slot.hotbar 0 bundle 1", "replaceitem entity @s slot.hotbar 1 dirt 4", "", "!bundleput 1 2 0", "", "!bundletag"]);
    lines.extend(owned(["summon allay 2.5 -26 0.5", "", "!interact allay", "", "!bundletag"]));
    waits(&mut lines, 40);
    with(lines, &["gamemode creative @s", "clear @s", "kill @e[type=!player]", ""])
}

const ANVIL_SHOWN: &[&str] = &["ItemStackResponse", "InventorySlot", "CommandOutput"];

/// Two enchanted iron swords merged in an anvil, the first laid as the input, for four pairs; the
/// name box holds the item's own name, as a client's does (BDS refused an empty list, status 9).
fn anvil() -> Vec<String> {
    let mut lines = with(floor(), &["setblock 2 -26 0 anvil", "gamemode survival @s", ""]);
    let pairs: [(&[&str], &[&str]); 4] = [
        (&["sharpness 2"], &["sharpness 3", "unbreaking 1"]),
        (&["sharpness 3"], &["sharpness 3"]),
        (&["sharpness 1", "looting 2"], &["smite 3", "looting 1"]),
        (&["smite 3"], &["sharpness 1", "knockback 2"]),
    ];
    for (first, second) in pairs {
        lines.extend(owned(["clear @s", "!forget", "xp 30L @s"]));
        lines.push(hold("iron_sword"));
        lines.extend(first.iter().map(|enchant| format!("enchant @s {enchant}")));
        lines.extend(owned(["", "!move 0 1"]));
        lines.push(hold("iron_sword"));
        lines.extend(second.iter().map(|enchant| format!("enchant @s {enchant}")));
        lines.extend(owned(["", "!click 2 0 0", "", "!ui 1 1", "!ui 0 2", "", "!anvilcraft! 0 Iron~Sword 8", "", "!inv", "!close", ""]));
    }
    with(lines, &["setblock 2 -26 0 air", "gamemode creative @s", "clear @s", ""])
}

const WEAR_SHOWN: &[&str] = &["InventorySlot", "InventoryContent", "MobArmorEquipment", "EntityEvent"];

/// Four iron pieces with Unbreaking III worn through a minute of six zombies' hits at night
/// (BDS's `/damage` wore no armour), the player kept alive by resistance and regeneration.
fn wear() -> Vec<String> {
    let mut lines = with(floor(), &["time set 18000", "gamemode survival @s", "effect @s resistance 999 3 true", "effect @s regeneration 999 4 true", "effect @s health_boost 999 4 true", ""]);
    for piece in ["helmet", "chestplate", "leggings", "boots"] {
        lines.extend([hold(&format!("iron_{piece}")), "enchant @s unbreaking 3".into(), "!use".into(), String::new()]);
    }
    lines.extend(["2.5 -26 0.5", "-1.5 -26 0.5", "0.5 -26 2.5", "0.5 -26 -1.5", "2.5 -26 2.5", "-1.5 -26 -1.5"].map(|at| format!("summon zombie {at}")));
    waits(&mut lines, 60);
    with(lines, &["kill @e[type=zombie]", "", "gamemode creative @s", "effect @s clear", "clear @s", "time set 6000", ""])
}

const BUCKETS_SHOWN: &[&str] = &["UpdateBlock", "UpdateBlockSynced", "UpdateSubchunkBlocks", "InventorySlot", "LevelSoundEvent", "LevelEvent", "AddEntity", "RemoveEntity", "MobEffect", "CompletedUsingItem", "ItemStackResponse"];

/// In survival: a water bucket used at the sky, an empty one at water; water into a fence, a slab
/// and stairs; a cauldron filled and emptied by water, powder snow and lava; cod, axolotl and
/// tadpole buckets poured and a cod taken back; powder snow set and taken; milk; dispensed buckets.
fn buckets() -> Vec<String> {
    let mut lines = with(floor(), &["gamemode survival @s", "tp CmdWatcher 12.5 -26 6.5", ""]);
    lines.extend([hold("water_bucket"), "!look 0 10 0.1".into(), "!use".into(), String::new(), "setblock 2 -26 0 water".into(), hold("bucket"), "!look 2 0.3 0".into(), "!use".into(), String::new()]);
    lines.extend(owned(["setblock 2 -26 0 air", "!look 2 0 0", ""]));
    for block in ["oak_fence", "smooth_stone_slab", "oak_stairs"] {
        lines.extend([format!("setblock 2 -26 0 {block}"), hold("water_bucket"), "!click 2 0 0".into(), String::new(), String::new(), "setblock 2 -26 0 air".into()]);
    }
    lines.push("setblock 2 -26 0 cauldron".into());
    for full in ["water_bucket", "powder_snow_bucket", "lava_bucket"] {
        lines.extend([hold(full), "!click 2 0 0".into(), String::new(), hold("bucket"), "!click 2 0 0".into(), String::new()]);
    }
    lines.extend(["setblock 2 -26 0 air".into(), hold("cod_bucket"), "!click 2 -1 0".into(), String::new(), String::new(), hold("bucket"), "!interact cod".into(), String::new()]);
    lines.extend(owned(["fill 1 -26 -1 3 -26 1 air", "kill @e[type=cod]"]));
    for (bucket, at) in [("axolotl_bucket", "2 -1 2"), ("tadpole_bucket", "2 -1 -2"), ("powder_snow_bucket", "-2 -1 2")] {
        lines.extend([hold(bucket), format!("!click {at}"), String::new(), String::new()]);
    }
    lines.extend(["kill @e[type=!player]".into(), hold("bucket"), "!click -2 0 2".into(), String::new(), "fill -8 -26 -8 16 -16 8 air".into(), String::new()]);
    lines.extend(["effect @s speed 30 1".into(), hold("milk_bucket"), String::new(), "!eat".into(), String::new(), String::new(), String::new()]);
    lines.extend(owned(["setblock -3 -26 3 dispenser [\"facing_direction\"=5]", "replaceitem block -3 -26 3 slot.container 0 water_bucket 1"]));
    for refill in ["bucket", "lava_bucket", "bucket"] {
        lines.extend([String::new(), "setblock -3 -27 3 redstone_block".into(), String::new(), "setblock -3 -27 3 stone".into(), String::new(), format!("replaceitem block -3 -26 3 slot.container 0 {refill} 1")]);
    }
    with(lines, &["fill -8 -26 -8 16 -16 8 air", "gamemode creative @s", "clear @s", "effect @s clear", "tp CmdWatcher 0.5 -26 0.5", ""])
}

const FISH_SHOWN: &[&str] = &["AddEntity", "AddItemEntity", "SetEntityData", "SetEntityMotion", "MoveEntityDelta", "RemoveEntity", "EntityEvent", "LevelSoundEvent", "SetEntityLink"];

/// In survival a Lure III rod cast into a pool and reeled at a bite, twice; then cast at a slowed
/// pig and reeled.
fn fish() -> Vec<String> {
    // The watcher out of the way: a hook hooks the first body it meets.
    let mut lines = with(floor(), &["tp @a[name=CmdWatcher] -2.5 -26 4.5", "fill 2 -31 -3 8 -27 3 stone", "fill 3 -30 -2 7 -27 2 water", "gamemode survival @s"]);
    lines.extend([hold("fishing_rod"), "enchant @s lure 3".into()]);
    lines.extend(owned(["tp @s 0.5 -26 0.5 -90 0", "!look 4.5 -0.5 0", ""]));
    lines.extend(owned(["!fish 45", "", "", "!fish 45", "", "", "summon pig 4.5 -26 -5.5", "effect @e[type=pig] slowness 999 255 true", "", "!look 4 0.5 -6", "", "!use", "", "", "!use", "", ""]));
    with(lines, &["fill 2 -31 -3 8 -27 3 stone", "fill 2 -27 -3 8 -27 3 stone", "gamemode creative @s", "clear @s", "kill @e[type=!player]", ""])
}
