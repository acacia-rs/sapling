//! The item drills of bds_command_probe: an elytra's glide and boost, buckets of powder snow,
//! fish and axolotls, books and lecterns, the loom, cartography table and beacon, and the small
//! items (goat horn, spyglass, cooldowns, maps).

use acacia_bot::Bot;
use acacia_bot::items::SlotRef;
use acacia_bot::proto::packets::LecternUpdate;
use acacia_bot::proto::types::Vec3i;
use acacia_bot::workstation::BeaconEffect;

pub const GLIDE_SHOWN: &[&str] = &[
    "SetEntityData", "PlayerArmorDamage", "MovementEffect", "AddEntity", "RemoveEntity", "PlayerHotbar", "InventorySlot", "InventoryContent", "MobEquipment", "MobArmorEquipment",
    "LevelSoundEvent", "EntityEvent", "CorrectPlayerMovePrediction", "UpdateAttributes", "ClientStartItemCooldown",
];
pub const BUCKETS_SHOWN: &[&str] = &[
    "UpdateBlock", "UpdateSubchunkBlocks", "LevelSoundEvent", "LevelEvent", "InventorySlot", "InventoryContent", "PlayerHotbar", "AddEntity", "RemoveEntity", "SetEntityData", "EntityEvent",
    "ItemStackResponse",
];
pub const BOOKS_SHOWN: &[&str] = &[
    "UpdateBlock", "BlockEntityData", "LevelSoundEvent", "LevelEvent", "InventorySlot", "InventoryContent", "PlayerHotbar", "AddItemEntity", "ContainerOpen", "ContainerClose", "LecternUpdate",
    "MobEquipment", "BlockEvent",
];
pub const STATIONS_SHOWN: &[&str] = &[
    "ContainerOpen", "ContainerClose", "ContainerSetData", "ItemStackResponse", "InventorySlot", "InventoryContent", "PlayerHotbar", "BlockEntityData", "LevelSoundEvent", "LevelEvent",
    "MobEffect", "ClientboundMapItemData", "UpdateBlock",
];
pub const SMALL_SHOWN: &[&str] = &[
    "LevelSoundEvent", "LevelEvent", "ClientStartItemCooldown", "SetEntityData", "InventorySlot", "InventoryContent", "PlayerHotbar", "AddEntity", "RemoveEntity", "CompletedUsingItem",
    "ClientboundMapItemData", "MobEquipment", "EntityEvent",
];

/// The block `offset` (three whole numbers) from the actor's feet, and the numbers after it.
fn at_and_rest(actor: &Bot, words: &str) -> Option<([i32; 3], Vec<String>)> {
    let words: Vec<&str> = words.split(' ').collect();
    let at = super::from_feet(actor, &words.get(..3)?.join(" "))?;
    Some((at, words[3..].iter().map(|word| word.to_string()).collect()))
}

/// Does an item drill's `!` line: `!glide` and `!unglide` start and stop a glide, `!boost` uses
/// the rocket in the hotbar, `!entity kind` uses the hand on the newest mob of that kind, `!write
/// a|b` writes pages in the book in hand and `!sign title` signs it, `!lectern x y z page count`
/// turns a lectern's page, `!loom x y z pattern` patterns the banner in hotbar 0 with the dye in 1,
/// `!beacon x y z` pays hotbar 0 for Speed, `!map` makes the empty map in hand, `!carto x y z`
/// copies the map in hotbar 0 with the empty one in 1, `!release` lets the use go. False else.
pub async fn act(actor: &mut Bot, line: &str) -> bool {
    let (word, rest) = line.split_once(' ').unwrap_or((line, ""));
    match word {
        "!entity" => {
            let kind = format!("minecraft:{rest}");
            match actor.state().entities.iter().filter(|entity| entity.kind == kind).map(|entity| entity.runtime_id).max() {
                Some(id) => println!("## used on: {:?}", actor.interact_entity(id).await),
                None => println!("## no {kind} to use on"),
            }
        }
        // Straight after a teleport up the bot still stands where it was: tried until it falls.
        "!glide" => {
            let mut tried = actor.start_gliding().await;
            for _ in 0..40 {
                if tried.is_ok() || actor.wait_ticks(1).await.is_err() {
                    break;
                }
                tried = actor.start_gliding().await;
            }
            println!("## glide: {tried:?}");
        }
        "!unglide" => println!("## unglide: {:?}", actor.stop_gliding().await),
        "!boost" => println!("## boost: {:?}", actor.boost_with_firework().await),
        "!write" => println!("## wrote: {:?}", actor.write_book(&rest.split('|').collect::<Vec<_>>()).await),
        "!sign" => println!("## signed: {:?}", actor.sign_book(rest).await),
        "!map" => println!("## map: {:?}", actor.make_map().await),
        "!release" => actor.release_item(),
        "!lectern" | "!loom" | "!beacon" | "!carto" => {
            let Some(([x, y, z], more)) = at_and_rest(actor, rest) else { return true };
            let number = |at: usize| more.get(at).and_then(|word| word.parse::<u8>().ok()).unwrap_or(0);
            match word {
                "!lectern" => drop(actor.client().send(&LecternUpdate { page: number(0), page_count: number(1), position: Vec3i { x, y, z } })),
                "!loom" => println!("## loomed: {:?}", actor.loom([x, y, z], SlotRef::Main(0), SlotRef::Main(1), more.first().map_or("bo", String::as_str), None).await),
                "!beacon" => println!("## paid: {:?}", actor.beacon([x, y, z], BeaconEffect::Speed, None, SlotRef::Main(0)).await),
                _ => println!("## copied: {:?}", actor.cartography([x, y, z], SlotRef::Main(0), Some(SlotRef::Main(1)), None, 1).await),
            }
        }
        _ => return false,
    }
    true
}

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

fn blank(lines: &mut Vec<String>, ticks: usize) {
    lines.extend(std::iter::repeat_n(String::new(), ticks));
}

/// A stone floor, the actor in survival looking east and down a little, the watcher behind it.
fn floor() -> Vec<String> {
    owned([
        "kill @e[type=!player]", "gamerule domobspawning false", "time set noon", "weather clear", "fill ~-3 ~-2 ~-3 ~8 ~-1 ~3 stone", "fill ~-3 ~ ~-3 ~8 ~4 ~3 air", "gamemode survival @s",
        "clear @s", "effect @s clear", "tp @s ~ ~ ~ -90 30", "tp @a[name=CmdWatcher] ~-2 ~ ~2", "",
    ])
}

/// The actor in survival with an elytra on: lifted 60 blocks, glides, boosts with a rocket,
/// stops in the air; then again with an elytra one use from breaking.
pub fn glide() -> Vec<String> {
    let mut lines = owned([
        "kill @e[type=!player]", "fill ~-4 ~-1 ~-4 ~40 ~-1 ~4 stone", "gamemode survival @s", "clear @s", "effect @s clear", "tp @a[name=CmdWatcher] ~-2 ~ ~2",
        "replaceitem entity @s slot.armor.chest 0 elytra", "replaceitem entity @s slot.weapon.mainhand 0 firework_rocket 3", "tp @s ~ ~60 ~ -90 10", "!glide",
    ]);
    blank(&mut lines, 4);
    lines.extend(owned(["!boost", "", "", "", "", "", "", "", "", "", "", "!unglide", "", ""]));
    lines.extend(owned(["replaceitem entity @s slot.armor.chest 0 elytra 1 430", "tp @s ~ ~60 ~ -90 10", "!glide", "", "", "", "", "", "", "", ""]));
    lines.extend(owned(["gamemode creative @s", "clear @s", ""]));
    lines
}

/// Powder snow taken up and put down; a cod and an axolotl scooped out of a pool and let out.
pub fn buckets() -> Vec<String> {
    let mut lines = floor();
    lines.extend(owned(["setblock ~2 ~ ~ powder_snow", "replaceitem entity @s slot.weapon.mainhand 0 bucket", "", "!click 2 0 0", "", "", "!click 3 -1 0", "", ""]));
    lines.extend(owned(["fill ~2 ~ ~ ~3 ~ ~ air", "setblock ~2 ~-1 ~ water", "summon cod ~2.5 ~-1 ~0.5", "replaceitem entity @s slot.weapon.mainhand 0 bucket", "", "!entity cod", "", ""]));
    lines.extend(owned(["replaceitem entity @s slot.weapon.mainhand 0 water_bucket", "", "!entity cod", "", "", "!click 4 -1 0", "", "", ""]));
    lines.extend(owned(["kill @e[type=!player]", "fill ~4 ~ ~ ~4 ~ ~ air", "summon axolotl ~2.5 ~-1 ~0.5", "replaceitem entity @s slot.weapon.mainhand 0 water_bucket", "", "!entity axolotl", "", ""]));
    lines.extend(owned(["!click 4 -1 0", "", "", "", "kill @e[type=!player]", "gamemode creative @s", "clear @s", ""]));
    lines
}

/// A book and quill written in and signed; the written book put on a lectern read by a
/// comparator, its pages turned, and the lectern broken.
pub fn books() -> Vec<String> {
    let mut lines = floor();
    lines.extend(owned(["replaceitem entity @s slot.weapon.mainhand 0 writable_book", "", "!write one|two|three", "", "", "!sign Tale", "", ""]));
    lines.extend(owned([
        "setblock ~2 ~ ~ lectern", "setblock ~3 ~ ~ unpowered_comparator [\"minecraft:cardinal_direction\"=\"west\",\"output_subtract_bit\"=false]", "setblock ~4 ~ ~ redstone_wire", "",
        "!click 2 0 0", "", "", "!lectern 2 0 0 1 3", "", "", "!lectern 2 0 0 2 3", "", "", "!close", "",
    ]));
    lines.extend(owned(["replaceitem entity @s slot.weapon.mainhand 0 writable_book", "!click 2 0 0", "", "", "!break 2 0 0", "", "", "gamemode creative @s", "clear @s", ""]));
    lines
}

/// A banner patterned at a loom; a beacon on a pyramid of one layer paid for Speed and waited
/// on; an empty map made, and copied at a cartography table.
pub fn stations() -> Vec<String> {
    let mut lines = floor();
    lines.extend(owned(["setblock ~2 ~ ~ loom", "replaceitem entity @s slot.hotbar 0 banner 1 15", "replaceitem entity @s slot.hotbar 1 red_dye 1", "", "!loom 2 0 0 bo", "", "!close", ""]));
    lines.extend(owned(["setblock ~2 ~ ~ air", "fill ~2 ~-1 ~-1 ~4 ~-1 ~1 iron_block", "setblock ~3 ~ ~ beacon", "replaceitem entity @s slot.hotbar 0 iron_ingot 1", "", "!beacon 3 0 0", "", "!close"]));
    blank(&mut lines, 10);
    lines.extend(owned(["setblock ~3 ~ ~ air", "replaceitem entity @s slot.hotbar 0 empty_map 1", "", "!map", "", "", ""]));
    lines.extend(owned(["setblock ~2 ~ ~ cartography_table", "replaceitem entity @s slot.hotbar 1 empty_map 1", "", "!carto 2 0 0", "", "!close", "gamemode creative @s", "clear @s", ""]));
    lines
}

/// Goat horns blown twice in a row and in another variant; a spyglass looked through and let
/// go; an ender pearl thrown twice in a row; chorus fruit eaten.
pub fn small() -> Vec<String> {
    let mut lines = floor();
    lines.extend(owned(["replaceitem entity @s slot.weapon.mainhand 0 goat_horn 1 0", "", "!use", "", "!use", "", "replaceitem entity @s slot.weapon.mainhand 0 goat_horn 1 3", "", "!use", "", ""]));
    lines.extend(owned(["replaceitem entity @s slot.weapon.mainhand 0 spyglass", "", "!use", "", "", "!release", "", ""]));
    lines.extend(owned(["tp @s ~ ~ ~ -90 -60", "replaceitem entity @s slot.weapon.mainhand 0 ender_pearl 2", "", "!use", "!use", "", "", "", "", ""]));
    lines.extend(owned(["replaceitem entity @s slot.weapon.mainhand 0 wind_charge 2", "", "!use", "!use", "", "", ""]));
    lines.extend(owned(["effect @s hunger 5 100", "replaceitem entity @s slot.weapon.mainhand 0 chorus_fruit 2", "", "", "", "!eat", "", "", "", "", "", "gamemode creative @s", "clear @s", ""]));
    lines
}

/// The lines and shown packets of the item drill `name`, if it is one.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "glide" => (glide(), GLIDE_SHOWN),
        "buckets" => (buckets(), BUCKETS_SHOWN),
        "books" => (books(), BOOKS_SHOWN),
        "stations" => (stations(), STATIONS_SHOWN),
        "small" => (small(), SMALL_SHOWN),
        _ => return None,
    })
}
