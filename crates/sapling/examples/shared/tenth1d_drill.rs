//! Drills of the tenth wave's commands (research/parity/TENTH-BRIEF.md "Group D"): what `/execute`
//! may run and does in other dimensions, `/time set` past a time, `/structure` in the Nether and the
//! End and over unloaded columns (`t1dexec`); `/weather query` second by second (`t1dweather`);
//! `/loot replace` on mobs and slot counts, tables `/loot` knows, `/playsound` to some hearers,
//! `/scriptevent`'s limit, `/recipe give *` (`t1dloot`); `tile.` keys and `seed`/`version` from a
//! command block (`t1dtile`); `/spreadplayers` landing and failing (`t1dspread`); `/place` feature
//! rules and structure names (`t1dplace`). On creature_drill.rs's floor (stone at y -27).

use acacia_bot::proto::packets::UnlockedRecipes;
use acacia_bot::proto::{Packet, RawPacket};

use super::creatures::{floor, waits, with};

const EXEC_SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "ChangeDimension", "AddEntity", "RemoveEntity", "SetTime"];
const WEATHER_SHOWN: &[&str] = &["LevelEvent"];
const LOOT_SHOWN: &[&str] = &["MobEquipment", "MobArmorEquipment", "InventorySlot", "InventoryContent", "AddItemEntity", "PlaySound", "UnlockedRecipesAll", "ScriptMessage"];
const TILE_SHOWN: &[&str] = &["BlockEntityData"];
const SPREAD_SHOWN: &[&str] = &["MovePlayer"];
const PLACE_SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "t1dexec" => (exec(), EXEC_SHOWN),
        "t1dweather" => (weather(), WEATHER_SHOWN),
        "t1dloot" => (loot(), LOOT_SHOWN),
        "t1dtile" => (tiles(), TILE_SHOWN),
        "t1dspread" => (spread(), SPREAD_SHOWN),
        "t1dplace" => (place(), PLACE_SHOWN),
        // The ground for `/place` typed at the console (CONSOLE), which a player may not run.
        // Two packs with a `probe/dup` each (tools/bds-packs-setup.sh DUP=1|2): which one runs.
        "t1dfunc" => (owned(&["", "", "function probe/dup", "function probe/second", ""]), &["Text"]),
        "t1drecipe" => (owned(&["", "recipe take @s *", "", "recipe give @s *", "", "recipe give @s *", ""]), &["UnlockedRecipesAll"]),
        "t1dplacec" => ([with(floor(), &["fill 20 -27 -8 60 -27 8 grass_block", "fill 20 -26 -8 60 -16 8 air"]), vec![String::new(); 30]].concat(), PLACE_SHOWN),
        _ => return None,
    })
}

/// UnlockedRecipes whole, as `type count: names` (command_show cuts long packets).
pub fn recipes(packet: &RawPacket) -> Option<String> {
    let unlocked = (packet.id == UnlockedRecipes::ID).then(|| packet.decode::<UnlockedRecipes>().ok()).flatten()?;
    Some(format!("UnlockedRecipes {:?} {}: {}", unlocked.unlock_type, unlocked.recipes.len(), unlocked.recipes.join(" ")))
}

fn owned(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|line| line.to_string()).collect()
}

fn exec() -> Vec<String> {
    let mut lines = with(floor(), &[
        "execute run place feature minecraft:oak_tree_feature 6 -26 0",
        "execute run reload",
        "execute run save query",
        "execute run setmaxplayers 10",
        "execute run permission list",
        "execute run allowlist list",
        "execute run changesetting allow-cheats true",
        "execute run wsserver",
        "execute run deop CmdWatcher",
        "execute run say still here",
        "execute in nether run setblock 0 64 0 stone",
        "execute in nether run testforblock 0 64 0 stone",
        "execute in nether run fill 0 65 0 1 65 1 glass",
        "execute in nether run summon pig 0 66 0",
        "execute in nether run testfor @e[type=pig]",
        "testfor @e[type=pig]",
        "execute in the_end run setblock 0 70 0 stone",
        "execute in the_end run testforblock 0 70 0 stone",
        "execute in nether run tp @s 0.5 67 0.5",
        "",
        "",
        "setblock 0 64 0 stone",
        "testforblock 0 64 0 stone",
        "execute in overworld run testforblock 0 -27 0 stone",
        "execute in nether run structure save t1dn 0 120 0 2 130 2",
        "execute in nether run structure save t1dn2 0 64 0 2 66 2",
        "execute in nether run structure load t1dn2 0 126 0",
        "execute in nether run structure load t1dn2 0 -2 0",
        "execute in overworld run tp @s 0.5 -26 0.5",
        "",
        "",
        "execute in the_end run structure save t1de 0 250 0 2 260 2",
        "execute in the_end run structure load mystructure:t1dn2 0 252 0",
        "execute in nether run structure load t1dn2 0 70 0",
        "structure load t1dn2 0 -26 4",
        "structure save t1du 5000 0 5000 5002 2 5002",
        "structure save t1du2 5000 0 5000 5002 2 5002 false memory",
        "structure load t1dn2 5000 0 5000",
        "structure delete t1du",
        "structure delete t1dn2",
    ]);
    lines.extend(owned(&[
        "time set 30000",
        "time query daytime",
        "time query gametime",
        "time query day",
        "time set day",
        "time query daytime",
        "time query gametime",
        "time query day",
        "time set sunrise",
        "time query gametime",
        "time set midnight",
        "time query gametime",
        "time set noon",
        "time query gametime",
        "time set 0",
        "time set midnight",
        "time query gametime",
        "time add 30000",
        "time set noon",
        "time query gametime",
        "time set sunset",
        "time query gametime",
        "time set night",
        "time query gametime",
    ]));
    lines
}

/// The answer to `/weather query` each second around changes of the weather.
fn weather() -> Vec<String> {
    let mut lines = floor();
    waits(&mut lines, 2);
    let queried = |lines: &mut Vec<String>, change: &str, seconds: usize| {
        lines.push(change.to_owned());
        lines.extend(std::iter::repeat_n("weather query".to_owned(), seconds));
    };
    queried(&mut lines, "weather thunder", 5);
    queried(&mut lines, "weather clear", 8);
    queried(&mut lines, "weather rain", 4);
    queried(&mut lines, "weather clear", 4);
    queried(&mut lines, "weather thunder 100", 8);
    queried(&mut lines, "weather rain", 3);
    queried(&mut lines, "toggledownfall", 3);
    lines.push("weather clear".into());
    lines
}

fn loot() -> Vec<String> {
    let mut lines = with(floor(), &[
        "summon cow 4 -26 0",
        "summon armor_stand 4 -26 3",
        "setblock 2 -26 -3 chest",
        "",
        "loot replace entity @e[type=cow] slot.weapon.mainhand 0 loot \"entities/zombie\"",
        "loot replace entity @e[type=cow] slot.weapon.mainhand 0 3 loot \"chests/simple_dungeon\"",
        "loot replace entity @e[type=cow] slot.armor.head 0 loot \"chests/simple_dungeon\"",
        "loot replace entity @e[type=cow] slot.hotbar 0 loot \"entities/cow\"",
        "loot replace entity @e[type=armor_stand] slot.weapon.offhand 0 loot \"chests/simple_dungeon\"",
        "loot replace entity @s slot.hotbar 0 9 loot \"chests/simple_dungeon\"",
        "loot replace entity @s slot.hotbar 7 9 loot \"chests/simple_dungeon\"",
        "loot replace entity @s slot.inventory 0 loot \"chests/simple_dungeon\"",
        "loot replace entity @s slot.inventory 0 40 loot \"chests/simple_dungeon\"",
        "loot replace entity @a slot.hotbar 0 2 loot \"chests/simple_dungeon\"",
        "loot replace block 2 -26 -3 slot.container 0 9 loot \"chests/simple_dungeon\"",
        "loot replace block 2 -26 -3 slot.container 25 9 loot \"chests/simple_dungeon\"",
        "loot replace block 2 -26 -3 slot.container 0 loot \"chests/simple_dungeon\"",
        "clear @a",
    ]);
    for table in [
        "gameplay/fishing", "gameplay/fishing/fish", "gameplay/fishing/junk", "gameplay/fishing/treasure", "gameplay/jungle_fishing", "entities/bat", "entities/cat_gift", "empty",
        "blocks/red_shrub", "blocks/white_wool_slab", "gameplay/entities/sniffer_seeds", "equipment/trial_chamber", "barrels/abandoned_camp_barrel", "entities/sheep_shear",
        "entities/zombie_equipment", "no/such", "entities/chicken.json", "loot_tables/entities/chicken",
    ] {
        lines.push(format!("loot spawn 0 -20 6 loot \"{table}\""));
    }
    lines.extend(owned(&[
        "kill @e[type=item]",
        "fill 58 -27 -2 62 -27 2 stone",
        "tp CmdWatcher 60.5 -26 0.5",
        "",
        "playsound random.click @a 0 -26 0",
        "playsound random.click @a 60 -26 0",
        "playsound random.click @a 30 -26 0",
        "playsound random.click @a 200 -26 0",
        "playsound random.click CmdWatcher 0 -26 0",
        "playsound random.click @a 0 -26 0 1 1 0.5",
        "playsound random.click @a",
        "tp CmdWatcher 0.5 -26 0.5",
        "",
        "scriptevent t1d:b hi",
        "scriptevent nonamespace hi",
        "scriptevent minecraft:x hi",
        "scriptevent t1d: hi",
        "scriptevent :x hi",
        "scriptevent t1d:empty",
    ]));
    for length in [2047, 2048, 2049, 4000] {
        lines.push(format!("scriptevent t1d:n{length} {}", "x".repeat(length)));
    }
    lines.extend(owned(&["recipe take @s *", "", "recipe give @s *", "", "recipe give @s *", "recipe take @s *", ""]));
    lines
}

/// Blocks named in `/testforblock` answers, an anvil by its damage; `seed` and `version` from a
/// command block and from the actor.
fn tiles() -> Vec<String> {
    let mut lines = floor();
    for (there, wanted) in [
        ("anvil", "stone"),
        ("anvil [\"damage\"=\"very_damaged\"]", "stone"),
        ("granite", "stone"),
        ("birch_log", "oak_log"),
        ("red_wool", "oak_planks"),
        ("smooth_stone_slab", "mossy_cobblestone"),
        ("zombie_head", "lit_furnace"),
        ("oak_leaves", "azure_bluet"),
        ("stone", "andesite_wall"),
    ] {
        lines.push(format!("setblock 3 -26 0 {there}"));
        lines.push(format!("testforblock 3 -26 0 {wanted}"));
    }
    lines.extend(owned(&[
        "setblock ~2 ~ ~ command_block [\"facing_direction\"=5]",
        "",
        "!cmdblock 2 0 0 0 1 0 0 0 1 seed",
        "setblock ~2 ~1 ~ redstone_block",
        "",
        "setblock ~2 ~1 ~ air",
        "!cmdblock 2 0 0 0 1 0 0 0 1 version",
        "setblock ~2 ~1 ~ redstone_block",
        "",
        "setblock ~2 ~1 ~ air",
        "seed",
        "version",
    ]));
    lines
}

/// The actor spread over a pad, then over pads of what it may not land on; cows spread too tight.
fn spread() -> Vec<String> {
    let mut lines = with(floor(), &["fill 34 -27 -6 46 -27 6 stone", "fill 34 -26 -6 46 -18 6 air", "spreadplayers 40 0 1 2 @s", "", "spreadplayers 40 0 1 2 @s", ""]);
    for cover in ["fire", "water", "lava", "magma_block", "cactus", "powder_snow", "sweet_berry_bush", "white_carpet", "glass", "campfire"] {
        lines.extend([format!("fill 34 -26 -6 46 -26 6 {cover}"), "tp @s 0.5 -26 0.5".into(), "spreadplayers 40 0 1 2 @s".into(), String::new(), "fill 34 -26 -6 46 -26 6 air".into()]);
    }
    lines.extend(owned(&["fill 34 -27 -6 46 -27 6 lava", "tp @s 0.5 -26 0.5", "spreadplayers 40 0 1 2 @s", "fill 34 -27 -6 46 -27 6 stone", "tp @s 0.5 -26 0.5"]));
    lines.extend((0..6).map(|i| format!("summon cow {} -26 {}", 2 + i, 4)));
    lines.extend(owned(&["", "spreadplayers 40 0 2 3 @e[type=cow]", "spreadplayers 40 0 3 4 @e[type=cow]", "spreadplayers 40 0 1 2 @e[type=cow]", "spreadplayers 40 0 1 5 @e[type=cow] -27", "kill @e[type=cow]"]));
    lines
}

/// Feature rules BDS lists and some it does not, structure names and options.
fn place() -> Vec<String> {
    let mut lines = with(floor(), &["fill 20 -27 -8 60 -27 8 grass_block", "fill 20 -26 -8 60 -16 8 air"]);
    for rule in ["minecraft:plains_surface_flowers_feature", "minecraft:overworld_surface_tall_grass_feature", "minecraft:mangrove_swamp_tall_mangrove_tree_feature", "minecraft:desert_after_surface_cactus_feature_rules", "minecraft:minecraft:savanna_mutated_surface_tall_grass_feature", "plains_surface_flowers_feature"] {
        lines.push(format!("place featurerule {rule} 30 -26 0"));
        lines.push(String::new());
    }
    lines.extend(owned(&[
        "place featurerule minecraft:plains_surface_flowers_feature",
        "place feature minecraft:beehive_feature 40 -26 0",
        "place structure minecraft:abandoned_camp_meadow 50 -26 0",
        "place structure minecraft:trail_ruins 50 -26 0 false false false banana",
        "place structure minecraft:trail_ruins 50 -26 0 false false false apply_waterlogging",
        "place structure minecraft:trail_ruins 50 -26 0 false false false ignore_waterlogging",
        "place structure minecraft:ancient_city 50 -26 0",
        "place structure minecraft:mineshaft 50 -26 0",
        "",
    ]));
    lines
}
