//! The fishing and brewing drills of bds_command_probe: a rod cast onto stone, into water and at
//! a pig, reeled at once, after a bite and with Lure; a brewing stand brewing three bottles.

pub const FISH_SHOWN: &[&str] = &[
    "AddEntity", "RemoveEntity", "EntityEvent", "SetEntityData", "SetEntityMotion", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityLink", "LevelSoundEvent", "LevelEvent", "LevelEventGeneric",
    "SpawnParticleEffect", "AddItemEntity", "TakeItemEntity", "SpawnExperienceOrb", "InventorySlot", "InventoryContent", "PlayerHotbar", "MobEquipment", "CompletedUsingItem", "Animate",
];

/// The actor in survival on a stone shore, a pool of water east of it, the watcher behind it out
/// of the way (a hook hooks the first body it meets): casts onto the stone and
/// reels in, casts into the water and reels in early, fishes until a bite twice (the second with
/// Lure III), casts at a pig and reels it in.
pub fn fish() -> Vec<String> {
    let mut lines: Vec<String> = [
        "kill @e[type=!player]", "gamerule domobspawning false", "weather clear", "time set noon", "fill ~-3 ~-4 ~-6 ~14 ~-1 ~6 stone", "fill ~-3 ~ ~-6 ~14 ~8 ~6 air", "fill ~3 ~-3 ~-5 ~13 ~-1 ~5 water",
        "gamemode survival @s", "clear @s", "tp @s ~ ~ ~ -90 0", "tp @a[name=CmdWatcher] ~-3 ~ ~4", "replaceitem entity @s slot.weapon.mainhand 0 fishing_rod", "",
    ]
    .map(String::from)
    .into();
    lines.extend(["!look 2 -1.5 0", "", "!use", "", "", "", "!use", "", ""].map(String::from));
    lines.extend(["!look 6 -1 0", "", "!use", "", "", "", "!use", "", ""].map(String::from));
    lines.extend(["!fish 60", "", "", ""].map(String::from));
    lines.extend(["!fish 60", "", "", ""].map(String::from));
    lines.extend(["replaceitem entity @s slot.weapon.mainhand 0 fishing_rod", "enchant @s lure 3", "", "!fish 60", "", "", ""].map(String::from));
    lines.extend(["summon pig ~4 ~ ~", "fill ~3 ~ ~-1 ~5 ~ ~1 stone", "tp @e[type=pig] ~4 ~1 ~", "", "!look 4 1 0", "", "!use", "", "", "!use", "", "", ""].map(String::from));
    lines.extend(["kill @e[type=!player]", "gamemode creative @s", "clear @s", ""].map(String::from));
    lines
}

pub const BREW_SHOWN: &[&str] = &["ContainerSetData", "InventorySlot", "InventoryContent", "ItemStackResponse", "LevelSoundEvent", "LevelEvent", "UpdateBlock", "BlockEvent", "ContainerOpen", "ContainerClose"];

/// The actor in survival opens a brewing stand east of it, puts in a blaze powder, a nether wart
/// and three water bottles and waits for the brew; then gunpowder over the awkward potions, then
/// sugar taken out after five seconds and put back, then glowstone, then a second blaze powder.
pub fn brew() -> Vec<String> {
    let mut lines: Vec<String> = [
        "kill @e[type=!player]", "fill ~-2 ~-1 ~-2 ~3 ~-1 ~2 stone", "fill ~-2 ~ ~-2 ~3 ~3 ~2 air", "gamemode survival @s", "clear @s", "tp @s ~ ~ ~ -90 0", "setblock ~1 ~ ~ brewing_stand",
        "replaceitem entity @s slot.hotbar 0 blaze_powder 2", "replaceitem entity @s slot.hotbar 1 nether_wart 1", "replaceitem entity @s slot.hotbar 2 potion 1 0", "replaceitem entity @s slot.hotbar 3 potion 1 0",
        "replaceitem entity @s slot.hotbar 4 potion 1 0", "replaceitem entity @s slot.hotbar 5 gunpowder 1", "replaceitem entity @s slot.hotbar 6 glowstone_dust 1", "replaceitem entity @s slot.hotbar 7 sugar 1", "",
        "!click 1 0 0", "", "!put 0 4 1", "", "!put 2 1", "!put 3 2", "!put 4 3", "", "!put 1 0", "",
    ]
    .map(String::from)
    .into();
    lines.extend(std::iter::repeat_n(String::new(), 22));
    lines.extend(["!put 5 0", ""].map(String::from));
    lines.extend(std::iter::repeat_n(String::new(), 22));
    lines.extend(["!put 7 0", "", "", "", "", "", "!take 0", "", "", "!put 7 0", ""].map(String::from));
    lines.extend(std::iter::repeat_n(String::new(), 22));
    lines.extend(["!put 6 0", ""].map(String::from));
    lines.extend(std::iter::repeat_n(String::new(), 22));
    lines.extend(["!put 0 4 1", "", "", "!close", "", "", "setblock ~1 ~ ~ air", "gamemode creative @s", "clear @s", ""].map(String::from));
    lines
}
