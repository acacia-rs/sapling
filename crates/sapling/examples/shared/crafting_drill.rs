//! The crafting and workstation drills of bds_command_probe: special recipes in a crafting table's
//! grid (`grid`), enchanting offers by bookshelf gap and item (`enchants`), anvil names (`anvilnames`),
//! banners kept, broken, blasted and loomed past six patterns (`banners`), a hopper at a brewing
//! stand (`hopperbrew`) and shulker boxes put into a bundle (`bundleshulker`). Requests: crafting_requests.rs.

#[path = "crafting_requests.rs"]
mod requests;

pub use requests::act;

pub const SHOWN: &[&str] = &[
    "ItemStackResponse", "InventorySlot", "InventoryContent", "ContainerOpen", "ContainerClose", "PlayerEnchantOptions", "AddItemEntity", "BlockEntityData", "UpdateBlock",
    "UpdateAttributes", "LevelEvent",
];

fn lines<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

/// A stone floor, the actor in creative at 0.5 ~ 0.5 looking east, the watcher behind it.
fn floor() -> Vec<String> {
    let mut out = crate::cmdblocks::floor();
    out.extend(lines(["tp @a[name=CmdWatcher] ~-2 ~ ~4", "effect @s clear", ""]));
    out
}

/// Inventory emptied, item entities killed, the request slots forgotten.
fn reset() -> Vec<String> {
    lines(["clear @s", "!forget", "kill @e[type=item]", ""])
}

/// A banner of colour `aux` patterned `bo` red at the loom two east and two south, into hotbar `dest`.
fn loomed(aux: u8, dest: u8) -> Vec<String> {
    let mut out = vec![format!("replaceitem entity @s slot.hotbar 0 banner 1 {aux}"), "replaceitem entity @s slot.hotbar 1 red_dye 1".into(), String::new()];
    // The bot's loom puts the result where the banner was (hotbar 0).
    out.extend(lines(["!loom 2 0 2 bo", ""]));
    out.extend([format!("!move 0 {dest}"), String::new()]);
    out
}

/// `!grid` at the crafting table two east, then the table closed.
fn crafted(grid: &str) -> Vec<String> {
    vec!["!click 2 0 0".into(), String::new(), grid.into(), String::new(), "!close".into(), String::new(), "!inv".into()]
}

/// Banner copies (a blank of the same colour, another colour, two patterned, two blanks), a
/// shield with a patterned, an ominous and a blank banner, a patterned shield again; book copies
/// (two at once, a copy of a copy, of a copy's copy, of a renamed book); map copies.
fn grid() -> Vec<String> {
    let mut out = floor();
    out.extend(lines(["setblock ~2 ~ ~ crafting_table", "setblock ~2 ~ ~2 loom", "setblock ~2 ~ ~-2 anvil", ""]));
    let steps: [(&[(u8, u8)], &[&str], &str); 6] = [
        (&[(15, 5)], &["replaceitem entity @s slot.hotbar 1 banner 1 15"], "!grid dup 1/2/3 5 1"),
        (&[(15, 5)], &["replaceitem entity @s slot.hotbar 1 banner 1 1"], "!grid dup 1/2 5 1"),
        (&[(15, 5), (15, 6)], &[], "!grid dup 1/2 5 6"),
        (&[(15, 5)], &["replaceitem entity @s slot.hotbar 1 banner 1 15", "replaceitem entity @s slot.hotbar 2 banner 1 15"], "!grid dup 1/2/3 5 1 2"),
        (&[(15, 5)], &["replaceitem entity @s slot.hotbar 1 shield"], "!grid! shield 1 1 5"),
        (&[], &["replaceitem entity @s slot.hotbar 1 shield", "replaceitem entity @s slot.hotbar 2 banner 1 4"], "!grid shield 1 1 2"),
    ];
    for (patterned, given, craft) in steps {
        out.extend(reset());
        for &(aux, dest) in patterned {
            out.extend(loomed(aux, dest));
        }
        out.extend(given.iter().map(|line| line.to_string()));
        out.extend(crafted(craft));
        if craft.starts_with("!grid!") {
            // The patterned shield kept lies in hotbar 0.
            out.extend(lines(["replaceitem entity @s slot.hotbar 4 banner 1 4", "!click 2 0 0", "", "!grid shield 1 0 4", "", "!close", ""]));
        }
    }
    out.extend(reset());
    out.extend(lines(["!ominous", "replaceitem entity @s slot.hotbar 1 shield", "", "!inv"]));
    out.extend(crafted("!grid shield 1 1 banner"));
    // Books: a written book, two copies at once, a copy of the copy, then of that one.
    out.extend(reset());
    out.extend(lines(["replaceitem entity @s slot.hotbar 0 writable_book", "", "!write one|two", "", "!sign Tale", "", "", "!inv"]));
    out.extend(lines(["replaceitem entity @s slot.hotbar 1 writable_book", "replaceitem entity @s slot.hotbar 2 writable_book", "!click 2 0 0", "", "!grid! book 1/2 0 1 2", ""]));
    out.extend(lines(["replaceitem entity @s slot.hotbar 2 writable_book", "", "!grid! book 1 0 2", "", "replaceitem entity @s slot.hotbar 4 writable_book", "", "!grid book 1 2 4", ""]));
    out.extend(lines(["!close", "", "!inv"]));
    // A renamed written book copied.
    out.extend(reset());
    out.extend(lines(["replaceitem entity @s slot.hotbar 0 writable_book", "", "!write one", "", "!sign Tale", "", "", "!click 2 0 -2", "", "!ui 0 1", "!anvilcraft! 0 Named 8", "!close", ""]));
    out.extend(lines(["replaceitem entity @s slot.hotbar 1 writable_book", "!click 2 0 0", "", "!grid book 1 8 1", "", "!close", ""]));
    // Maps: a filled map with one empty map, then with two.
    for empties in [1, 2] {
        out.extend(reset());
        out.extend(lines(["replaceitem entity @s slot.hotbar 0 empty_map", "", "!map", "", "", "!inv"]));
        let given: Vec<String> = (1..=empties).map(|slot| format!("replaceitem entity @s slot.hotbar {slot} empty_map")).collect();
        out.extend(given);
        let cells: Vec<String> = (0..=empties).map(|slot| slot.to_string()).collect();
        out.extend(crafted(&format!("!grid map 1/{}/{} {}", empties, empties + 1, cells.join(" "))));
    }
    out.extend(reset());
    out
}

/// An enchanting table two east with two bookshelves behind a gap: a sword laid in for each
/// block in the gap's lower cell; then other items with the gap open and shut; `/enchant` on
/// shears, a spear and a skull.
fn enchants() -> Vec<String> {
    let mut out = floor();
    out.extend(lines(["setblock ~2 ~ ~ enchanting_table", "setblock ~4 ~ ~ bookshelf", "setblock ~4 ~1 ~ bookshelf", "setblock ~3 ~-1 ~ dirt", "replaceitem entity @s slot.hotbar 0 diamond_sword", ""]));
    out.extend(lines(["!click 2 0 0", ""]));
    let look = |out: &mut Vec<String>| out.extend(lines(["!ui 0 14", "", "!uiback 14 0", ""]));
    for gap in ["air", "stone", "torch", "white_carpet", "snow_layer", "short_grass", "glass", "air"] {
        out.push(format!("setblock ~3 ~ ~ {gap}"));
        out.push(String::new());
        look(&mut out);
    }
    for gap in ["air", "stone"] {
        out.push(format!("setblock ~3 ~ ~ {gap}"));
        for item in ["bow", "crossbow", "book", "mace", "fishing_rod", "wooden_spear", "shears", "golden_helmet"] {
            out.push(format!("replaceitem entity @s slot.hotbar 0 {item}"));
            out.push(String::new());
            look(&mut out);
        }
    }
    out.extend(lines(["!close", "", "replaceitem entity @s slot.hotbar 0 shears", ""]));
    out.extend(lines(["enchant @s silk_touch", "enchant @s efficiency", "enchant @s unbreaking", "enchant @s sharpness", "enchant @s fortune"]));
    out.extend(lines(["replaceitem entity @s slot.hotbar 0 wooden_spear", "", "enchant @s sharpness", "enchant @s smite", "enchant @s looting", "enchant @s knockback", "enchant @s fire_aspect"]));
    out.extend(lines(["replaceitem entity @s slot.hotbar 0 skeleton_skull", "", "enchant @s binding", "enchant @s vanishing", "enchant @s protection", "enchant @s respiration"]));
    out.extend(lines(["replaceitem entity @s slot.hotbar 0 carved_pumpkin", "", "enchant @s binding", "enchant @s unbreaking", "", "clear @s"]));
    out
}

/// Anvil renames in survival with 30 levels: to the item's own name, with two names and index 1,
/// then 0, with index -1, with no names, and a named sword's name cleared.
fn anvil_names() -> Vec<String> {
    let mut out = floor();
    out.extend(lines(["setblock ~2 ~ ~ anvil", "gamemode survival @s", "xp 30L @s", ""]));
    for craft in ["0 Diamond~Sword 8", "1 First|Second 8", "0 First|Second 8", "-1 Named 8", "0 _ 8", "0 diamond~sword 8"].map(|craft| format!("!anvilcraft {craft}")) {
        out.extend(reset());
        out.extend(lines(["replaceitem entity @s slot.hotbar 0 diamond_sword", "", "!click 2 0 0", "", "!ui 0 1"]));
        // BDS keeps a refused input in the anvil after a close; it is taken back first.
        out.extend([craft, "!uiback 1 7".into(), "!close".into(), String::new()]);
    }
    out.extend(reset());
    out.extend(lines(["replaceitem entity @s slot.hotbar 0 diamond_sword", "", "!click 2 0 0", "", "!ui 0 1", "!anvilcraft! 0 Named 8", "", "!ui 8 1", "!anvilcraft 0 '' 7", "!close", ""]));
    out.extend(lines(["gamemode creative @s", "clear @s", ""]));
    out
}

/// A loomed banner placed and broken in survival; an ominous one too; a seventh pattern loomed;
/// two patterned banners and a plain one blown up together.
fn banners() -> Vec<String> {
    let mut out = floor();
    out.extend(lines(["setblock ~1 ~ ~3 loom", "fill ~-1 ~-1 ~-5 ~9 ~-1 ~5 obsidian", "gamemode survival @s", ""]));
    out.extend(lines(["replaceitem entity @s slot.hotbar 0 banner 1 15", "replaceitem entity @s slot.hotbar 1 red_dye 1", "", "!loom 1 0 3 bo", "", "!inv"]));
    out.extend(lines(["!place 3 -1 0", "", "", "!break 3 0 0", "", "", "kill @e[type=item]", ""]));
    out.extend(reset());
    out.extend(lines(["gamemode creative @s", "!ominous", "gamemode survival @s", "", "!inv", "!place 3 -1 0", "", "", "!break 3 0 0", "", "", "kill @e[type=item]", ""]));
    out.extend(reset());
    out.extend(lines(["replaceitem entity @s slot.hotbar 0 banner 1 15", ""]));
    for _ in 0..7 {
        out.extend(lines(["replaceitem entity @s slot.hotbar 1 red_dye 1", "", "!loom 1 0 3 bo", ""]));
    }
    out.extend(lines(["!inv", "!drop 0 1", "", "kill @e[type=item]"]));
    out.extend(reset());
    out.extend(lines(["gamemode creative @s", "replaceitem entity @s slot.hotbar 0 banner 1 15", "replaceitem entity @s slot.hotbar 1 red_dye 1", "", "!loom 1 0 3 bo", ""]));
    out.extend(lines(["!place 3 -1 -1", "", "!place 3 -1 1", "", "setblock ~3 ~ ~ standing_banner", "", "summon tnt ~4 ~ ~"]));
    out.extend(std::iter::repeat_n(String::new(), 6));
    out.extend(lines(["kill @e[type=item]", "clear @s", ""]));
    out
}

/// A hopper pointing into a brewing stand whose bottle slots 1 and 2 hold a glass bottle; the
/// stand opened afterwards, then the hopper.
fn hopper_brew() -> Vec<String> {
    let mut out = floor();
    out.extend(lines(["setblock ~2 ~ ~ brewing_stand", "replaceitem block ~2 ~ ~ slot.container 1 glass_bottle 1", "replaceitem block ~2 ~ ~ slot.container 2 glass_bottle 1", ""]));
    out.extend(lines(["setblock ~3 ~ ~ hopper [\"facing_direction\"=4]", "replaceitem block ~3 ~ ~ slot.container 0 glass_bottle 5", "", "", "", "", ""]));
    out.extend(lines(["!click 2 0 0", "", "!close", "", "!click 3 0 0", "", "!close", ""]));
    out
}

/// Shulker boxes and cobblestone put into a bundle.
fn bundle_shulker() -> Vec<String> {
    let mut out = floor();
    out.extend(lines(["gamemode survival @s", "replaceitem entity @s slot.hotbar 0 bundle 1", "replaceitem entity @s slot.hotbar 1 undyed_shulker_box 1", "replaceitem entity @s slot.hotbar 2 red_shulker_box 1"]));
    out.extend(lines(["replaceitem entity @s slot.hotbar 3 cobblestone 4", "", "", "!bundletag", "!bundleput 1 1 0", "", "!bundleput 2 1 0", "", "!bundleput 3 4 0", "", "!bundletag", "gamemode creative @s", "clear @s", ""]));
    out
}

/// The lines and shown packets of the crafting drill `name`, if it is one.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    let lines = match name {
        "grid" => grid(),
        "enchants" => enchants(),
        "anvilnames" => anvil_names(),
        "banners" => banners(),
        "hopperbrew" => hopper_brew(),
        "bundleshulker" => bundle_shulker(),
        _ => return None,
    };
    Some((lines, SHOWN))
}
