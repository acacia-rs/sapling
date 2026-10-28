//! Drills of game rules that change what blocks and the recipe book do: `projectilesCanBreakBlocks`
//! (shots at a decorated pot, a chorus flower and pointed dripstone) and `recipesUnlock` (what
//! picking items up unlocks).

/// Packets shown of these drills.
pub const RULES_SHOWN: &[&str] = &["UpdateBlock", "LevelSoundEvent", "LevelEvent", "AddEntity", "AddItemEntity", "RemoveEntity", "UnlockedRecipes", "GameRulesChanged", "ItemStackResponse"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some((
        match name {
            "shotblocks" => shot_blocks(),
            "recipeunlock" => recipe_unlock(),
            "recipeunlock2" => recipe_unlock_more(),
            "limitedcraft" => limited_craft(),
            _ => return None,
        },
        RULES_SHOWN,
    ))
}

fn floor() -> Vec<String> {
    ["kill @e[type=!player]", "gamerule domobspawning false", "gamemode creative @s", "clear @s", "tp @s 0.5 ~ 0.5 -90 0", "fill ~-1 ~-1 ~-6 ~10 ~-1 ~8 stone", "fill ~-1 ~ ~-6 ~10 ~6 ~8 air", "time set noon", ""]
        .map(String::from)
        .into()
}

/// A dispenser two east of the actor, `z` across, fires `ammo` east at whatever stands three
/// further on, powered by a redstone block put on top of it.
fn fire(lines: &mut Vec<String>, z: i32, ammo: &str) {
    lines.extend([
        format!("setblock ~2 ~ ~{z} dispenser [\"facing_direction\"=5]"),
        format!("replaceitem block ~2 ~ ~{z} slot.container 0 {ammo} 1"),
        String::new(),
        format!("setblock ~2 ~1 ~{z} redstone_block"),
        String::new(),
        String::new(),
        String::new(),
    ]);
}

/// Arrows at a decorated pot, at a chorus flower on end stone and at a stalagmite of pointed
/// dripstone (which BDS breaks for a trident alone, 0x14660dfd0); then with
/// `projectilescanbreakblocks` off, an arrow at a second pot and a snowball at a second flower.
pub fn shot_blocks() -> Vec<String> {
    let mut lines = floor();
    lines.extend(["setblock ~5 ~ ~-4 decorated_pot", "setblock ~5 ~-1 ~-1 end_stone", "setblock ~5 ~ ~-1 chorus_flower", "setblock ~5 ~ ~2 pointed_dripstone [\"hanging\"=false,\"dripstone_thickness\"=\"tip\"]"].map(String::from));
    fire(&mut lines, -4, "arrow");
    fire(&mut lines, -1, "arrow");
    fire(&mut lines, 2, "arrow");
    lines.extend(["gamerule projectilescanbreakblocks false", "setblock ~5 ~ ~5 decorated_pot", "setblock ~5 ~-1 ~7 end_stone", "setblock ~5 ~ ~7 chorus_flower"].map(String::from));
    fire(&mut lines, 5, "arrow");
    fire(&mut lines, 7, "snowball");
    lines.extend(["gamerule projectilescanbreakblocks true", "kill @e[type=!player]", ""].map(String::from));
    lines
}

/// The actor in survival, its book reset, is given an oak log, cobblestone and an iron ingot one
/// at a time; then with `recipesunlock` off, a diamond and a stick; then with it on again, gold.
pub fn recipe_unlock() -> Vec<String> {
    let mut lines = floor();
    lines.extend(["gamemode survival @s", "recipe take @s *", "", "give @s oak_log", "", "", "give @s cobblestone", "", "", "give @s iron_ingot", "", ""].map(String::from));
    lines.extend(["gamerule recipesunlock false", "", "give @s diamond", "", "", "give @s stick", "", "", "gamerule recipesunlock true", "", "give @s gold_ingot", "", "", "gamemode creative @s", "clear @s"].map(String::from));
    lines
}

/// What else unlocks: a full stack, then single items of new kinds one at a time (the chest's
/// "many items"); the same item again, before and after `/recipe take` of one it unlocked; an item
/// put in the offhand; one given in creative; standing in water (boats).
pub fn recipe_unlock_more() -> Vec<String> {
    let mut lines = floor();
    lines.extend(["gamemode survival @s", "recipe take @s *", ""].map(String::from));
    lines.extend(["give @s dirt 64", "", "give @s dirt 1", ""].map(String::from));
    for item in ["sand", "gravel", "clay_ball", "flint", "feather", "bone", "string", "leather", "paper", "egg", "apple", "wheat_seeds"] {
        lines.extend([format!("give @s {item}"), String::new()]);
    }
    lines.extend(["clear @s", "", "give @s oak_log", "", "give @s oak_log", "", "recipe take @s minecraft:oak_planks", "", "give @s oak_log", "", "clear @s", ""].map(String::from));
    lines.extend(["replaceitem entity @s slot.weapon.offhand 0 iron_ingot", "", "", "gamemode creative @s", "give @s copper_ingot", "", "", "gamemode survival @s", ""].map(String::from));
    lines.extend(["fill ~3 ~ ~3 ~5 ~1 ~5 water", "", "tp @s ~3.5 ~ ~3.5", "", "", "", "tp @s 0.5 ~ 0.5", "", "gamemode creative @s", "clear @s"].map(String::from));
    lines
}

/// With `doLimitedCrafting` on, sticks crafted from planks while the stick recipe is known, then
/// after `/recipe take` of it; then with the rule off again.
pub fn limited_craft() -> Vec<String> {
    let mut lines = floor();
    lines.extend(["gamemode survival @s", "recipe take @s *", "gamerule dolimitedcrafting true", "", "give @s oak_planks 8", "", "!craft stick", "", ""].map(String::from));
    lines.extend(["recipe take @s minecraft:stick", "", "!craft stick", "", "", "gamerule dolimitedcrafting false", "", "!craft stick", "", "", "gamemode creative @s", "clear @s"].map(String::from));
    lines
}
