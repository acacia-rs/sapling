//! Drills of blocks that keep something: bells, conduits, chiseled bookshelves and beehives.

/// Packets shown of these drills.
pub const KEEPER_SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "BlockEntityData", "BlockEvent", "InventorySlot", "LevelEvent", "LevelSoundEvent", "AddItemEntity", "AddEntity", "RemoveEntity"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some((
        match name {
            "bells" => bells(),
            "conduits" => conduits(),
            "bookshelves" => bookshelves(),
            "hives" => hives(),
            _ => return None,
        },
        KEEPER_SHOWN,
    ))
}

fn floor() -> Vec<String> {
    ["kill @e[type=!player]", "gamerule domobspawning false", "gamemode creative @s", "tp @s 0.5 ~ 0.5 -90 0", "fill ~-2 ~-1 ~-6 ~10 ~-1 ~6 stone", "fill ~-2 ~ ~-6 ~10 ~6 ~6 air", "clear @s", ""]
        .map(String::from)
        .into()
}

fn waits(lines: &mut Vec<String>, seconds: usize) {
    lines.extend(std::iter::repeat_n(String::new(), seconds));
}

fn end(mut lines: Vec<String>) -> Vec<String> {
    lines.extend(["gamemode creative @s", "fill ~-2 ~ ~-6 ~10 ~6 ~6 air", "kill @e[type=!player]", "clear @s", ""].map(String::from));
    lines
}

/// A standing bell two east, rung by the actor's click, again a tick later, then by a block of
/// redstone put beside it; a bell hung from a stone's side, a standing bell and a bell hung
/// under a stone, each clicked on one face after another.
pub fn bells() -> Vec<String> {
    let mut lines = floor();
    lines.extend(["setblock ~2 ~ ~0 bell [\"attachment\"=\"standing\",\"direction\"=0]", "", "!click 2 0 0", "!click 2 0 0"].map(String::from));
    waits(&mut lines, 3);
    lines.extend(["setblock ~2 ~ ~1 redstone_block", "", "", "setblock ~2 ~ ~1 air", ""].map(String::from));
    lines.extend(["setblock ~3 ~1 ~-3 stone", "setblock ~2 ~1 ~-3 bell [\"attachment\"=\"side\",\"direction\"=1]", "", "!click 2 1 -3 east"].map(String::from));
    waits(&mut lines, 3);
    for face in ["west", "north", "south", "up", "down"] {
        lines.extend([format!("!click 2 1 -3 {face}"), String::new(), String::new()]);
    }
    lines.push("setblock ~2 ~ ~3 bell [\"attachment\"=\"standing\",\"direction\"=0]".into());
    for face in ["west", "east", "north", "south"] {
        lines.extend([format!("!click 2 0 3 {face}"), String::new(), String::new()]);
    }
    lines.extend(["setblock ~6 ~2 ~0 stone", "setblock ~6 ~1 ~0 bell [\"attachment\"=\"hanging\",\"direction\"=0]", ""].map(String::from));
    for face in ["west", "north", "down"] {
        lines.extend([format!("!click 6 1 0 {face}"), String::new(), String::new()]);
    }
    waits(&mut lines, 4);
    end(lines)
}

/// A conduit in a cube of water in a shell of prismarine, watched while it wakes; then most of
/// the shell taken away. The conduit stands in water: the actor empties a bucket on it before
/// the shell goes up around it.
pub fn conduits() -> Vec<String> {
    let mut lines = floor();
    lines.extend(
        [
            "setblock ~5 ~2 ~0 conduit",
            "replaceitem entity @s slot.weapon.mainhand 0 water_bucket",
            "!click 5 2 0 west",
            "",
            "fill ~3 ~ ~-2 ~7 ~4 ~2 prismarine outline",
            "fill ~4 ~1 ~-1 ~6 ~3 ~1 water replace air",
        ]
        .map(String::from),
    );
    waits(&mut lines, 10);
    lines.extend(["fill ~3 ~ ~-2 ~7 ~4 ~-2 glass", "fill ~3 ~ ~2 ~7 ~4 ~2 glass", "fill ~3 ~4 ~-2 ~7 ~4 ~2 glass"].map(String::from));
    waits(&mut lines, 8);
    end(lines)
}

/// A chiseled bookshelf two east facing the actor: a book put in by a click on its face, a
/// comparator behind it; a second book on the filled slot; an empty hand on the emptied slot; a
/// book in again, taken by an empty hand, and in again; the shelf broken.
pub fn bookshelves() -> Vec<String> {
    let mut lines = floor();
    lines.extend(
        [
            "setblock ~2 ~ ~0 chiseled_bookshelf [\"direction\"=1]",
            "setblock ~3 ~ ~0 unpowered_comparator [\"minecraft:cardinal_direction\"=\"west\"]",
            "setblock ~4 ~ ~0 redstone_wire",
            "",
            "replaceitem entity @s slot.weapon.mainhand 0 book 4",
            "!click 2 0 0 west",
            "",
            "replaceitem entity @s slot.weapon.mainhand 0 enchanted_book 1",
            "!click 2 0 0 west",
            "",
            "replaceitem entity @s slot.weapon.mainhand 0 air",
            "!click 2 0 0 west",
            "",
            "replaceitem entity @s slot.weapon.mainhand 0 book 4",
            "!click 2 0 0 west",
            "",
            "replaceitem entity @s slot.weapon.mainhand 0 air",
            "!click 2 0 0 west",
            "",
            "replaceitem entity @s slot.weapon.mainhand 0 book 4",
            "!click 2 0 0 west",
            "",
            "!break 2 0 0",
            "",
        ]
        .map(String::from),
    );
    end(lines)
}

/// A beehive full of honey sheared, a bee nest full of honey bottled, two beehives with a bee
/// near each at night; one broken in creative, the other's bee let out at noon.
pub fn hives() -> Vec<String> {
    let mut lines = floor();
    lines.extend(
        [
            "gamemode survival @s",
            "setblock ~2 ~ ~-2 beehive [\"honey_level\"=5,\"direction\"=1]",
            "setblock ~2 ~ ~2 bee_nest [\"honey_level\"=5,\"direction\"=1]",
            "",
            "replaceitem entity @s slot.weapon.mainhand 0 shears",
            "!click 2 0 -2 west",
            "",
            "replaceitem entity @s slot.weapon.mainhand 0 glass_bottle",
            "!click 2 0 2 west",
            "",
            "time set midnight",
            "setblock ~6 ~ ~0 beehive [\"direction\"=1]",
            "summon bee ~7 ~1 ~0",
            "setblock ~6 ~ ~3 beehive [\"direction\"=1]",
            "summon bee ~7 ~1 ~3",
        ]
        .map(String::from),
    );
    waits(&mut lines, 20);
    lines.extend(["gamemode creative @s", "!break 6 0 0", "", "", "time set noon"].map(String::from));
    waits(&mut lines, 4);
    end(lines)
}
