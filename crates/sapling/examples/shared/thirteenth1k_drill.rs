//! The thirteenth wave's group K drills (research/parity/THIRTEENTH-BRIEF.md): a `can_place_on` stack
//! placed in survival beside a plain one, to tell BDS's adventure refusal from the bot's held item
//! (acacia's bot sends its held item with empty lists), and long `can_destroy` lists whose order is
//! compared between runs (`k1lock`).

use super::creatures::{floor, with};

const LOCK_SHOWN: &[&str] = &["InventorySlot", "InventoryContent", "UpdateBlock", "CommandOutput"];

/// Names whose legacy ids have one block each, so no list widens.
const NAMED: &str = r#""glass","gravel","bookshelf","obsidian","ice","clay","pumpkin","netherrack","glowstone","melon_block","mycelium","end_stone","emerald_block","redstone_block","quartz_ore","slime","hay_block","coal_block","packed_ice","bone_block","magma","nether_wart_block","observer","honey_block","tuff","calcite","amethyst_block","moss_block","mud","deepslate""#;

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "k1lock" => (lock(), LOCK_SHOWN),
        "k1cmds" => (commands(), CMDS_SHOWN),
        "k1jig" => (jigsaws(), JIG_SHOWN),
        "k1gate" => (gateway(), GATE_SHOWN),
        "k1cool" => (cooldowns(), COOL_SHOWN),
        "k1spec" => (spectators(), SPEC_SHOWN),
        _ => return None,
    })
}

const SPEC_SHOWN: &[&str] = &["ChangeDimension", "CommandOutput", "UpdatePlayerGameType"];

/// A spectator stood in a lit nether portal for 8 seconds, then in an end portal for 4; a
/// creative player in the nether portal after, to show the portal works.
fn spectators() -> Vec<String> {
    let mut lines = with(floor(), &["fill 2 -27 2 5 -23 2 obsidian", "fill 3 -26 2 4 -24 2 portal [\"portal_axis\"=\"x\"]", "setblock 3 -27 6 end_portal", "", "gamemode spectator @s", "tp @s 3.5 -26 2.5"]);
    lines.extend(std::iter::repeat_n(String::new(), 8));
    lines.extend(["execute in overworld run tp @s 3.5 -27 6.5", "", "", "", ""].map(String::from));
    lines.extend(["execute in overworld run tp @s 0.5 -26 0.5", "gamemode creative @s", "", "", "tp @s 3.5 -26 2.5"].map(String::from));
    lines.extend(std::iter::repeat_n(String::new(), 8));
    with(lines, &["execute in overworld run tp @s 0.5 -26 0.5", "", "", "fill 2 -27 2 5 -23 6 air", "fill 2 -27 2 5 -27 6 stone", ""])
}

const COOL_SHOWN: &[&str] = &["ClientStartItemCooldown", "AddEntity", "CommandOutput"];

/// Pearls thrown 1, 3 and 6 seconds apart, then wind charges 1 and 3 apart, the actor put back
/// before each throw.
fn cooldowns() -> Vec<String> {
    let mut lines = with(floor(), &["gamerule falldamage false", "replaceitem entity @s slot.weapon.mainhand 0 ender_pearl 16", ""]);
    for gap in [0, 1, 3, 6] {
        lines.extend(std::iter::repeat_n(String::new(), gap));
        lines.extend(["tp @s 0.5 -26 0.5 -90 -60".into(), "!use".into()]);
    }
    lines.extend(["".into(), "".into(), "replaceitem entity @s slot.weapon.mainhand 0 wind_charge 16".into(), "".into()]);
    for gap in [0, 1, 3] {
        lines.extend(std::iter::repeat_n(String::new(), gap));
        lines.extend(["tp @s 0.5 -26 0.5 -90 -60".into(), "!use".into()]);
    }
    with(lines, &["", "", "gamerule falldamage true", "kill @e[type=!player]", "clear @s", "tp @s 0.5 -26 0.5", ""])
}

const GATE_SHOWN: &[&str] = &["MovePlayer", "AddEntity", "RemoveEntity", "MoveEntityAbsolute", "SetEntityLink", "SetEntityMotion", "ChangeDimension", "CommandOutput"];

/// Seed 20261018's worldgen gateway at (1953, 66, 181): a pig and a summoned pearl put in it,
/// then the actor riding a horse moved into it, each a cooldown apart.
fn gateway() -> Vec<String> {
    let mut lines = with(Vec::new(), &["kill @e[type=!player]", "gamerule domobspawning false", "execute in the_end run tp @s 1956.5 66 181.5", "", "", "", "", "", ""]);
    lines.extend(["summon pig 1953.5 66 181.5", "", "", "", "summon ender_pearl 1953.5 66.5 181.5", "", "", ""].map(String::from));
    lines.extend(["summon horse 1956.5 66 184.5 0 0 minecraft:on_tame", "", "ride @s start_riding @e[type=horse,c=1]", "", "execute as @e[type=horse] run tp @s 1953.5 66 181.5", "", "", ""].map(String::from));
    with(lines, &["ride @s stop_riding", "", "execute in overworld run tp @s 0.5 -26 0.5", "", ""])
}

const JIG_SHOWN: &[&str] = &["AddEntity", "UpdateSubchunkBlocks", "UpdateBlock", "BlockEntityData", "CommandOutput"];

/// `/place jigsaw` names with and without the namespace (k1jig: every unnamespaced one was a
/// generation error), then a village centre alone with each option: its jigsaws counted by a fill
/// that clears them, its villagers by a kill.
fn jigsaws() -> Vec<String> {
    let mut lines = with(floor(), &["", ""]);
    for (pool, target, at) in [("", "", "48 -60 0"), ("minecraft:", "minecraft:", "48 -60 0"), ("minecraft:", "", "48 -60 48"), ("", "minecraft:", "0 -60 48")] {
        lines.extend([format!("place jigsaw {pool}trail_ruins/tower {target}road_anchor 3 {at}"), String::new()]);
    }
    for ([x, z], options) in [([-48, 0], ""), ([0, -48], " true"), ([-48, -48], " false false"), ([48, -48], " true true")] {
        let (low, high) = ([x - 16, -63, z - 16], [x + 15, -44, z + 15]);
        lines.extend([format!("place jigsaw minecraft:village/plains/town_centers minecraft:bottom 1 {x} -60 {z}{options}"), String::new(), String::new()]);
        lines.extend([format!("fill {} {} {} {} {} {} air replace jigsaw", low[0], low[1], low[2], high[0], high[1], high[2]), "kill @e[type=villager_v2]".into(), "kill @e[type=iron_golem]".into()]);
    }
    with(lines, &["kill @e[type=!player]", ""])
}

const CMDS_SHOWN: &[&str] = &["CommandOutput", "Text", "SetTitle", "InventorySlot"];

/// Rawtext parts with two keys each way round, `/replaceitem` amounts and data, scoreboard tests
/// on holders without the objective, a `*` score in `/titleraw`, and `/tag` by a type name.
fn commands() -> Vec<String> {
    let raw = |parts: &str| format!(r#"tellraw @s {{"rawtext":[{parts}]}}"#);
    let mut lines = with(floor(), &["scoreboard objectives add k1 dummy", "scoreboard objectives add k1b dummy", "scoreboard players set @s k1 7", "scoreboard players set zed k1b 3", "scoreboard players set alpha k1 4", "scoreboard players set beta k1 5"]);
    for parts in [
        r#"{"text":"T","translate":"commands.give.success"}"#,
        r#"{"translate":"commands.give.success","text":"T"}"#,
        r#"{"selector":"@s","text":"T"}"#,
        r#"{"text":"T","selector":"@s"}"#,
        r#"{"translate":"commands.give.success","selector":"@s"}"#,
        r#"{"score":{"name":"@s","objective":"k1"},"text":"T"}"#,
        r#"{"score":{"name":"@s","objective":"k1"},"selector":"@s"}"#,
        r#"{"score":{"name":"@s","objective":"k1"},"translate":"commands.give.success"}"#,
        r#"{"score":{"name":"*","objective":"k1"}}"#,
    ] {
        lines.push(raw(parts));
    }
    lines.extend([
        r#"titleraw @s actionbar {"rawtext":[{"score":{"name":"*","objective":"k1"}}]}"#,
        r#"titleraw @s actionbar {"rawtext":[{"score":{"name":"alpha","objective":"k1"}}]}"#,
        "scoreboard players test zed k1 0 9",
        "scoreboard players test nobody k1 0 9",
        "scoreboard players test alpha k1 0 9",
        "scoreboard players list",
        "scoreboard players list zed",
        "scoreboard players list *",
        "replaceitem entity @s slot.hotbar 0 stone 0",
        "replaceitem entity @s slot.hotbar 0 stone -1",
        "replaceitem entity @s slot.hotbar 0 stone 65",
        "replaceitem entity @s slot.hotbar 0 ender_pearl 17",
        "replaceitem entity @s slot.hotbar 0 ender_pearl 64",
        "replaceitem entity @s slot.hotbar 0 diamond_sword 2",
        "replaceitem entity @s slot.hotbar 0 stone 1 99",
        "replaceitem entity @s slot.hotbar 0 wool 1 20",
        "replaceitem entity @s slot.hotbar 0 wool 1 5",
        "replaceitem entity @s slot.hotbar 0 diamond_sword 1 30",
        "replaceitem entity @s slot.hotbar 0 diamond_sword 1 3000",
        "replaceitem entity @s slot.hotbar 0 stone 1 -1",
        "setblock 3 -26 0 chest",
        "replaceitem block 3 -26 0 slot.container 0 ender_pearl 20",
        "replaceitem block 3 -26 0 slot.container 0 stone 99",
        "replaceitem block 3 -26 0 slot.container 27 stone 1",
        "summon pig 4 -26 2",
        "",
        "tag pig add k1t",
        "tag @e[type=pig] list",
        "tag @e[type=pig] add k1t",
        "tag @e[type=pig] list",
        "scoreboard objectives remove k1",
        "scoreboard objectives remove k1b",
        "setblock 3 -26 0 air",
        "kill @e[type=!player]",
        "clear @s",
        "",
    ].map(String::from));
    lines
}

/// A listed and a plain stack placed in survival, the plain one in adventure; then two long lists.
fn lock() -> Vec<String> {
    let listed = r#"replaceitem entity @s slot.weapon.mainhand 0 stone 4 0 {"minecraft:can_place_on":{"blocks":["grass_block"]}}"#;
    let mut lines = with(floor(), &["setblock 2 -27 0 grass_block", "gamemode survival @s", listed, "", "!place 2 -1 0", "", "setblock 2 -26 0 air"]);
    lines.extend(["replaceitem entity @s slot.weapon.mainhand 0 stone 4", "", "!place 2 -1 0", "", "setblock 2 -26 0 air", "gamemode adventure @s", "", "!place 2 -1 0", ""].map(String::from));
    lines.push(format!(r#"replaceitem entity @s slot.hotbar 1 diamond_pickaxe 1 0 {{"minecraft:can_destroy":{{"blocks":[{NAMED}]}}}}"#));
    lines.push(format!(r#"replaceitem entity @s slot.hotbar 2 dirt 1 0 {{"minecraft:can_place_on":{{"blocks":[{NAMED},"stone","dirt"]}}}}"#));
    with(lines, &["", "gamemode creative @s", "clear @s", "setblock 2 -27 0 stone", "setblock 2 -26 0 air", ""])
}
