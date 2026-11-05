//! Group P's block-use drills: dispensers at targets and a dripleaf (`p2shoot`), a tripwire and plates
//! (`p2trip`), tools on blocks (`p2tools`), sand falling on small blocks and an anvil (`p2fall`), lava
//! meeting water and water beside upside-down shapes (`p2liquid`), lightning on a rod over copper and a
//! jukebox (`p2copper`), a furnace's output slot (`p2furnace`).

use super::owned;
use super::super::creatures::{floor, waits};

const SHOOT_SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "AddEntity", "RemoveEntity", "LevelEvent", "LevelSoundEvent", "CommandOutput"];
const TRIP_SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "AddEntity", "AddItemEntity", "RemoveEntity", "CommandOutput"];
const USE_SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "AddEntity", "AddItemEntity", "RemoveEntity", "LevelEvent", "LevelSoundEvent", "CommandOutput"];
const FURNACE_SHOWN: &[&str] = &["ContainerOpen", "ItemStackResponse", "InventorySlot", "InventoryContent", "CommandOutput"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "p2shoot" => (shooting(), SHOOT_SHOWN),
        "p2trip" => (tripping(), TRIP_SHOWN),
        "p2tools" => (tools(), USE_SHOWN),
        "p2fall" => (falling(), USE_SHOWN),
        "p2liquid" => (liquids(), USE_SHOWN),
        "p2copper" => (copper(), USE_SHOWN),
        "p2furnace" => (furnace(), FURNACE_SHOWN),
        _ => return None,
    })
}

/// Dispensers at x 4 facing east, each fired by a block of redstone set behind it: arrows at a target
/// (z -4), snowballs at one (z -2), fireworks at one (z 0), arrows at a big dripleaf (z 2), a lamp behind
/// each at x 9; then an empty
/// dispenser set beside a lit block of redstone (z 5).
fn shooting() -> Vec<String> {
    let mut lines = floor();
    lines.push("setblock 8 -27 2 dirt".to_owned());
    for (z, item) in [(-4, "arrow"), (-2, "snowball"), (0, "firework_rocket"), (2, "arrow")] {
        lines.push(format!("setblock 4 -26 {z} dispenser [\"facing_direction\"=5]"));
        lines.push(format!("replaceitem block 4 -26 {z} slot.container 0 {item} 2"));
        lines.push(if z == 2 { "setblock 8 -26 2 big_dripleaf [\"big_dripleaf_head\"=true]".to_owned() } else { format!("setblock 8 -26 {z} target") });
        lines.push(format!("setblock 9 -26 {z} redstone_lamp"));
    }
    lines.push(String::new());
    for z in [-4, -2, 0, 2] {
        lines.extend([format!("setblock 3 -26 {z} redstone_block"), String::new(), String::new(), format!("setblock 3 -26 {z} air"), String::new()]);
    }
    lines.extend(owned(["setblock 4 -26 5 redstone_block", "setblock 5 -26 5 dispenser [\"facing_direction\"=5]", ""]));
    waits(&mut lines, 3);
    lines.extend(owned(["kill @e[type=!player]", "fill 3 -26 -4 10 -24 5 air", "setblock 8 -27 2 stone", ""]));
    lines
}

/// Tripwire hooks on stone at x 2 and 6 (z -4), a lamp behind the west stone, string between; chicken
/// loot, an arrow and a pig dropped on the string; then chicken loot on a stone and a wooden plate, and
/// the actor standing on the stone one as a spectator.
fn tripping() -> Vec<String> {
    let mut lines = floor();
    lines.extend(owned(["setblock 2 -26 -4 stone", "setblock 6 -26 -4 stone", "setblock 1 -26 -4 redstone_lamp", "replaceitem entity @s slot.weapon.mainhand 0 tripwire_hook 4", ""]));
    lines.extend(owned(["tp @s 4.5 -26 -1.5 180 0", "", "!place 2 0 -2 west", "!place -2 0 -2 east", ""]));
    lines.extend(owned(["replaceitem entity @s slot.weapon.mainhand 0 string 4", "!place 0 -1 -2", "tp @s 0.5 -26 0.5 -90 0", ""]));
    lines.extend(owned(["loot spawn 4.5 -25.5 -3.5 loot \"entities/chicken\"", "", "", "", "kill @e[type=item]", "", "summon arrow 4.5 -25 -3.5", "", "", "kill @e[type=arrow]", ""]));
    lines.extend(owned(["summon pig 4.5 -26 -3.5", "", "", "kill @e[type=pig]", "", "fill 1 -26 -4 6 -26 -4 air", ""]));
    lines.extend(owned(["setblock 4 -26 2 stone_pressure_plate", "setblock 6 -26 2 wooden_pressure_plate", "", "loot spawn 4.5 -25.5 2.5 loot \"entities/chicken\"", "loot spawn 6.5 -25.5 2.5 loot \"entities/chicken\""]));
    lines.extend(owned(["", "", "kill @e[type=item]", "", "gamemode spectator @s", "tp @s 4.5 -26 2.5", "", "", "tp @s 0.5 -26 0.5 -90 0", "gamemode creative @s", "fill 4 -26 2 6 -26 2 air", "clear @s", ""]));
    lines
}

/// A shovel on the tops of seven blocks set into the floor (x 2), then an axe on seven more (x 3).
fn tools() -> Vec<String> {
    const DUG: [&str; 7] = ["dirt_with_roots", "coarse_dirt", "podzol", "mycelium", "dirt", "farmland", "grass_path"];
    const CUT: [&str; 7] = ["crimson_stem", "warped_hyphae", "bamboo_block", "oxidized_copper", "waxed_copper", "cherry_wood", "pumpkin"];
    let mut lines = floor();
    for (x, item, blocks) in [(2, "iron_shovel", DUG), (3, "iron_axe", CUT)] {
        for (block, z) in blocks.iter().zip(-3..) {
            lines.push(format!("setblock {x} -27 {z} {block}"));
        }
        lines.push(format!("replaceitem entity @s slot.weapon.mainhand 0 {item} 1"));
        lines.extend((-3..=3).map(|z| format!("!click {x} -1 {z}")));
        lines.push(String::new());
    }
    lines.extend(owned(["kill @e[type=item]", "fill 2 -27 -3 3 -27 3 stone", "clear @s", ""]));
    lines
}

/// Sand let fall from y -22 onto a torch, a poppy, a rail, short grass, a carpet, a bottom slab and a snow
/// layer (x 4, z -3 to 3); an anvil from the same height onto the floor (x 8).
fn falling() -> Vec<String> {
    const SMALL: [&str; 7] = ["torch", "poppy", "rail", "short_grass", "white_carpet", "oak_slab", "snow_layer"];
    let mut lines = floor();
    for (block, z) in SMALL.iter().zip(-3..) {
        lines.push(format!("setblock 4 -26 {z} {block}"));
    }
    lines.extend(owned(["", "fill 4 -22 -3 4 -22 3 sand", "setblock 8 -22 0 anvil", ""]));
    waits(&mut lines, 3);
    lines.extend(owned(["kill @e[type=!player]", "fill 4 -26 -3 8 -22 3 air", ""]));
    lines
}

/// Lava two cells from water (z -4); water beside an upside-down stair (z 0) and a top slab (z 2); a slab
/// placed into water and broken (z 4).
fn liquids() -> Vec<String> {
    let mut lines = floor();
    lines.extend(owned(["setblock 4 -26 -4 lava", "setblock 6 -26 -4 water", ""]));
    waits(&mut lines, 3);
    lines.extend(owned(["setblock 4 -26 0 oak_stairs [\"upside_down_bit\"=true,\"weirdo_direction\"=0]", "setblock 4 -26 2 smooth_stone_slab [\"minecraft:vertical_half\"=\"top\"]", ""]));
    lines.extend(owned(["setblock 3 -26 0 water", "setblock 3 -26 2 water", ""]));
    waits(&mut lines, 3);
    lines.extend(owned(["fill 1 -26 -8 9 -24 8 air", "", "setblock 4 -26 4 water", "replaceitem entity @s slot.weapon.mainhand 0 oak_slab 4", "!place 4 -1 4", "", "!break 4 0 4", ""]));
    waits(&mut lines, 2);
    lines.extend(owned(["fill 1 -26 -8 9 -24 8 air", "clear @s", ""]));
    lines
}

/// Lightning summoned onto a rod on oxidized copper with weathered and oxidized copper around (z -2); a
/// cat disc put in a jukebox and taken out (z 2); a sneaking axe click on oxidized copper (x 2, z 0).
fn copper() -> Vec<String> {
    let mut lines = floor();
    lines.extend(owned(["setblock 4 -27 -2 oxidized_copper", "setblock 5 -27 -2 oxidized_copper", "setblock 3 -27 -2 weathered_copper", "setblock 4 -27 -3 exposed_copper", "setblock 4 -26 -2 lightning_rod", ""]));
    lines.extend(owned(["summon lightning_bolt 4.5 -26 -2.5", ""]));
    waits(&mut lines, 3);
    lines.extend(owned(["setblock 3 -26 2 jukebox", "replaceitem entity @s slot.weapon.mainhand 0 music_disc_cat 1", "!click 3 0 2 west", "", "", "!click 3 0 2 west", ""]));
    lines.extend(owned(["setblock 2 -27 0 oxidized_copper", "replaceitem entity @s slot.weapon.mainhand 0 iron_axe 1", "!sneak", "", "!click 2 -1 0", "!unsneak", ""]));
    lines.extend(owned(["kill @e[type=!player]", "fill 2 -26 -3 5 -24 2 air", "fill 2 -27 -3 5 -27 0 stone", "clear @s", ""]));
    lines
}

/// The actor opens a furnace and puts cobblestone into its output slot.
fn furnace() -> Vec<String> {
    let mut lines = floor();
    lines.extend(owned(["setblock 3 -26 0 furnace", "replaceitem entity @s slot.weapon.mainhand 0 cobblestone 10", "", "!click 3 0 0 west", "", "!put 0 2", "", "!close", ""]));
    lines.extend(owned(["setblock 3 -26 0 air", "clear @s", ""]));
    lines
}
