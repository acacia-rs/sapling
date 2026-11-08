//! The fourteenth wave's group B drills (research/parity/FOURTEENTH-BRIEF.md). On creature_drill.rs's floor
//! (stone at y -27, the actor at 0.5 -26 0.5 facing east): a creative actor eats and drinks beside a sensor
//! (`b1eat`); a sneaking survival actor's shield takes `/damage` of each cause, a zombie and a skeleton, then
//! it rides a boat with and without the shield (`b1shield`, and a mount, hits and a dismount alone: `b1ride`);
//! clicks on a cow and a sheep seen by the watcher (`b1click`); cows, chickens, a zombie and the actor dropped
//! onto farmland (`b1tread`), cows walked off a ledge they stood on (`b1tread2`) and dropped from heights
//! (`b1tread3`; its rows at negative cells are misplaced, `format!("{x}.5")`); a sensor beside a wandering pig,
//! a hurt and killed cow, a hurt actor and TNT behind a wall (`b1sculk`), and pigs and zombies walked to a
//! sensor (`b1steps`). On the flat ground
//! (stand at y -60; sapling `SAPLING_DRILL_WORLD=lowflat`): stems, composters, pale moss carpets, mangrove
//! propagules and pitcher crops broken by `setblock ... destroy` (`b1stem`).

use super::creatures::{floor, waits, with};

const EAT_SHOWN: &[&str] = &[
    "MobEffect", "CompletedUsingItem", "InventorySlot", "InventoryContent", "MovePlayer", "LevelSoundEvent", "UpdateAttributes", "EntityEvent", "SetEntityData", "UpdateBlock", "BlockEntityData",
    "LevelEventGeneric", "PlayerStartItemCooldown", "CommandOutput",
];
const SHIELD_SHOWN: &[&str] = &[
    "SetEntityData", "LevelSoundEvent", "InventoryContent", "InventorySlot", "UpdateAttributes", "EntityEvent", "SetEntityMotion", "SetEntityLink", "Animate", "CorrectPlayerMovePrediction",
    "CommandOutput", "AddEntity", "RemoveEntity", "MobEffect",
];
const CLICK_SHOWN: &[&str] = &["Animate", "EntityEvent", "LevelSoundEvent", "InventorySlot", "InventoryContent", "AddItemEntity", "SetEntityData", "CommandOutput"];
const TREAD_SHOWN: &[&str] = &["UpdateBlock", "CommandOutput", "AddEntity", "RemoveEntity", "LevelSoundEvent", "EntityEvent"];
const SCULK_SHOWN: &[&str] = &["UpdateBlock", "BlockEntityData", "LevelEventGeneric", "LevelSoundEvent", "LevelEvent", "CommandOutput", "AddEntity", "RemoveEntity", "MoveEntityDelta", "EntityEvent"];
const STEM_SHOWN: &[&str] = &["AddItemEntity", "UpdateBlock", "CommandOutput"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "b1eat" => (eat(), EAT_SHOWN),
        "b1shield" => (shield(), SHIELD_SHOWN),
        "b1click" => (click(), CLICK_SHOWN),
        "b1ride" => (ride(), SHIELD_SHOWN),
        "b1tread" => (tread(), TREAD_SHOWN),
        "b1tread2" => (tread_after_standing(), TREAD_SHOWN),
        "b1tread3" => (tread_by_height(), TREAD_SHOWN),
        "b1sculk" => (sculk(), SCULK_SHOWN),
        "b1steps" => (steps(), SCULK_SHOWN),
        "b1stem" => (stem(), STEM_SHOWN),
        _ => return None,
    })
}

fn hold(item: &str) -> String {
    format!("replaceitem entity @s slot.weapon.mainhand 0 {item}")
}

/// A creative actor with full hunger eats and drinks each a while apart, a sensor 3 off; put back after the
/// chorus fruit.
fn eat() -> Vec<String> {
    let mut lines = with(floor(), &["setblock 3 -26 2 sculk_sensor", ""]);
    waits(&mut lines, 2);
    for item in ["golden_apple 2", "chorus_fruit 2", "bread 2", "potion 1 14", "honey_bottle 2", "milk_bucket 1"] {
        lines = with(lines, &[&hold(item), "", "!eat"]);
        waits(&mut lines, 4);
        lines = with(lines, &["tp @s 0.5 -26 0.5 -90 0", ""]);
    }
    with(lines, &["effect @s clear", "clear @s", "setblock 3 -26 2 air", ""])
}

const HEAL: &str = "effect @s instant_health 1 4";

/// Sneaking with a shield in the off hand, facing east: `/damage` of each cause by a stand ahead; a zombie
/// and a skeleton come at it; then in a boat, a stand ahead and behind, a zombie, and with no shield.
fn shield() -> Vec<String> {
    let by = |cause: &str| format!("damage @s 3 {cause} entity @e[type=armor_stand,c=1]");
    let mut lines = with(floor(), &["tp CmdWatcher 0.5 -26 6.5", "gamemode survival @s", "replaceitem entity @s slot.weapon.offhand 0 shield", "summon armor_stand 2.5 -26 0.5", "!sneak", "", ""]);
    for cause in ["entity_attack", "thorns", "fireworks", "magic", "ram_attack", "sonic_boom", "mace_smash", "projectile"] {
        lines = with(lines, &[&by(cause), "", HEAL, ""]);
    }
    for (kind, far, seconds) in [("zombie", "4.5", 6), ("skeleton", "9.5", 6)] {
        lines = with(lines, &["kill @e[type=!player]", HEAL, &format!("summon {kind} {far} -26 0.5")]);
        waits(&mut lines, seconds);
    }
    lines = with(lines, &["kill @e[type=!player]", HEAL, "!unsneak", "", "summon boat 0.5 -26 0.5", "", "!mount boat", "", "!look 2 1.5 0", "", "summon armor_stand 2.5 -26 0.5", ""]);
    lines = with(lines, &[&by("entity_attack"), "", HEAL, "kill @e[type=armor_stand]", "summon armor_stand -1.5 -26 0.5", "", &by("entity_attack"), "", HEAL, "kill @e[type=armor_stand]"]);
    lines = with(lines, &["summon zombie 3.5 -26 0.5"]);
    waits(&mut lines, 6);
    lines = with(lines, &["kill @e[type=zombie]", HEAL, "replaceitem entity @s slot.weapon.offhand 0 air", "summon armor_stand 2.5 -26 0.5", "", &by("entity_attack"), "", "!dismount", ""]);
    with(lines, &["kill @e[type=!player]", "gamemode creative @s", "effect @s clear", "clear @s", ""])
}

/// `b1shield` again without the knockback causes: sneaking, a fire-proof skeleton's shots; then in a boat
/// summoned at its feet, a stand's hit ahead and behind, a fire-proof zombie, and with no shield.
fn ride() -> Vec<String> {
    let by = |cause: &str| format!("damage @s 3 {cause} entity @e[type=armor_stand,c=1]");
    let mut lines = with(floor(), &["tp CmdWatcher 0.5 -26 6.5", "gamemode survival @s", "replaceitem entity @s slot.weapon.offhand 0 shield", "!sneak", "", ""]);
    lines = with(lines, &["summon skeleton 9.5 -26 0.5", "effect @e[type=skeleton] fire_resistance 100 0 true"]);
    waits(&mut lines, 8);
    lines = with(lines, &["kill @e[type=!player]", HEAL, "!unsneak", "", "tp @s 0.5 -26 0.5 -90 0", "summon boat 0.5 -26 0.5", "", "!mount boat", "", "summon armor_stand 2.5 -26 0.5", ""]);
    lines = with(lines, &[&by("entity_attack"), "", HEAL, "kill @e[type=armor_stand]", "summon armor_stand -1.5 -26 0.5", "", &by("entity_attack"), "", HEAL, "kill @e[type=armor_stand]"]);
    lines = with(lines, &["summon zombie 3.5 -26 0.5", "effect @e[type=zombie] fire_resistance 100 0 true"]);
    waits(&mut lines, 6);
    lines = with(lines, &["kill @e[type=zombie]", HEAL, "replaceitem entity @s slot.weapon.offhand 0 air", "summon armor_stand 2.5 -26 0.5", "", &by("entity_attack"), "", "!dismount", ""]);
    with(lines, &["kill @e[type=!player]", "gamemode creative @s", "effect @s clear", "clear @s", ""])
}

/// A held cow and sheep beside the actor clicked with wheat, shears, a bucket, a stick and nothing, the
/// watcher 4 off.
fn click() -> Vec<String> {
    let mut lines = with(floor(), &[
        "tp CmdWatcher 0.5 -26 4.5", "summon cow 2.5 -26 0.5 0 0 minecraft:entity_spawned", "summon sheep 2.5 -26 -1.5 0 0 minecraft:entity_spawned", "effect @e[type=!player] slowness 200 255 true", "",
    ]);
    for (item, kind) in [("wheat 4", "cow"), ("shears", "sheep"), ("bucket", "cow"), ("stick", "cow"), ("air", "cow")] {
        lines = with(lines, &[&hold(item), "", &format!("!interact {kind}"), "", ""]);
    }
    with(lines, &["kill @e[type=!player]", "clear @s", ""])
}

/// Each falls from 4 over its own wet farmland cell in the floor; read back after.
fn tread() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher 0.5 -26 -7.5", "gamerule mobgriefing true", ""]);
    let cases = [
        ("cow", 2), ("cow", 4), ("cow", 6), ("chicken", 8), ("chicken", 10), ("zombie", 12), ("sheep", 14),
        ("creative", 2), ("creative", 4), ("creative", 6), ("survival", 8), ("griefless", 10), ("griefless", 12),
    ];
    for (i, (who, x)) in cases.into_iter().enumerate() {
        let z = if i < 7 { -4 } else { 3 };
        let cell = format!("{x} -27 {z}");
        lines = with(lines, &[&format!("setblock {cell} farmland [\"moisturized_amount\"=7]"), ""]);
        let drop = format!("{x}.5 -23 {z}.5");
        lines.extend(match who {
            "creative" | "survival" => vec![format!("gamemode {who} @s"), format!("tp @s {drop}")],
            "griefless" => vec!["gamerule mobgriefing false".into(), format!("summon cow {drop}")],
            kind => vec![format!("summon {kind} {drop}")],
        });
        waits(&mut lines, 3);
        lines = with(lines, &[&format!("testforblock {cell} farmland"), "kill @e[type=!player]", "gamemode creative @s", "gamerule mobgriefing true", "tp @s 0.5 -26 0.5 -90 0", ""]);
    }
    with(lines, &["fill -8 -27 -8 16 -27 8 stone", ""])
}

/// Cows that stood on stone 3 over their farmland, the stone then taken away, and a cow and a zombie
/// summoned 9 over theirs; read back after.
fn tread_after_standing() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher 0.5 -26 -7.5", "gamerule mobgriefing true", ""]);
    for (kind, x, over) in [("cow", 2, 3), ("cow", 4, 3), ("cow", 6, 3), ("cow", 8, 9), ("zombie", 10, 9)] {
        let cell = format!("{x} -27 0");
        let ledge = format!("{x} {} 0", -27 + over);
        lines = with(lines, &[&format!("setblock {cell} farmland [\"moisturized_amount\"=7]"), &format!("setblock {ledge} stone"), &format!("summon {kind} {x}.5 {} 0.5", -26 + over)]);
        lines.extend(if over == 3 { vec![String::new(), String::new(), format!("setblock {ledge} air")] } else { vec![format!("setblock {ledge} air")] });
        waits(&mut lines, 3);
        lines = with(lines, &[&format!("testforblock {cell} farmland"), "kill @e[type=!player]", ""]);
    }
    with(lines, &["fill -8 -27 -8 16 -27 8 stone", ""])
}

/// Twelve held cows a row stood on stone 2, 3, 4, 5, 6 and 8 over farmland, the stone then taken away
/// at once: the farmland trodden is read from the UpdateBlocks.
fn tread_by_height() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher 0.5 -26 7.5", "tp @s -7.5 -26 7.5 -90 0", "gamerule mobgriefing true"]);
    let rows = [(2, -7), (3, -5), (4, -3), (5, -1), (6, 1), (8, 3)];
    for (over, z) in rows {
        lines.push(format!("fill -7 -27 {z} 15 -27 {z} farmland [\"moisturized_amount\"=7]"));
        lines.push(format!("fill -7 {} {z} 15 {} {z} stone", -27 + over, -27 + over));
        lines.extend((-7..=15).step_by(2).map(|x| format!("summon cow {x}.5 {} {z}.5 0 0 minecraft:entity_spawned", -26 + over)));
    }
    lines = with(lines, &["effect @e[type=cow] slowness 100 255 true", ""]);
    lines.extend(rows.map(|(over, z)| format!("fill -7 {} {z} 15 {} {z} air", -27 + over, -27 + over)));
    waits(&mut lines, 5);
    with(lines, &["kill @e[type=!player]", "fill -8 -27 -8 16 -27 8 stone", "tp @s 0.5 -26 0.5 -90 0", ""])
}

/// A sensor at 0 -26 0, both players over 15 off: a pig let wander beside it and sent away; a held cow hurt,
/// then killed; the actor put 3 off, then hurt; TNT 7 off behind an obsidian wall.
fn sculk() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher 15.5 -26 7.5", "tp @s 15.5 -26 -7.5 -90 0", "setblock 0 -26 0 sculk_sensor", ""]);
    waits(&mut lines, 3);
    lines = with(lines, &["summon pig 3.5 -26 0.5"]);
    waits(&mut lines, 12);
    lines = with(lines, &["tp @e[type=pig] 40.5 -26 0.5", ""]);
    waits(&mut lines, 3);
    lines = with(lines, &["kill @e[type=pig]", "summon cow 3.5 -26 0.5 0 0 minecraft:entity_spawned", "effect @e[type=cow] slowness 100 255 true"]);
    waits(&mut lines, 4);
    lines = with(lines, &["damage @e[type=cow] 1"]);
    waits(&mut lines, 4);
    lines = with(lines, &["kill @e[type=cow]"]);
    waits(&mut lines, 4);
    lines = with(lines, &["tp @s 3.5 -26 3.5 -90 0"]);
    waits(&mut lines, 4);
    lines = with(lines, &["gamemode survival @s", "", "damage @s 1"]);
    waits(&mut lines, 3);
    lines = with(lines, &["gamemode creative @s", "tp @s 15.5 -26 -7.5 -90 0", "fill 4 -26 -2 4 -24 2 obsidian"]);
    waits(&mut lines, 4);
    lines = with(lines, &["summon tnt 7.5 -26 0.5"]);
    waits(&mut lines, 7);
    with(lines, &["kill @e[type=item]", "fill -8 -27 -8 16 -27 8 stone", "fill -8 -26 -8 16 -20 8 air", ""])
}

/// Twice, a fire-proof zombie summoned 23 west of the survival actor walks east to it between sensors
/// 3 either side of its way.
fn steps() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher 0.5 -26 7.5", "setblock 0 -26 3 sculk_sensor", "setblock 0 -26 -3 sculk_sensor", "gamemode survival @s", "tp @s 15.5 -26 0.5 -90 0", "effect @s resistance 100 4 true"]);
    waits(&mut lines, 3);
    for _ in 0..2 {
        lines = with(lines, &["summon zombie -7.5 -26 0.5", "effect @e[type=zombie] fire_resistance 100 0 true"]);
        waits(&mut lines, 12);
        lines = with(lines, &["kill @e[type=zombie]", ""]);
        waits(&mut lines, 3);
    }
    with(lines, &["gamemode creative @s", "effect @s clear", "setblock 0 -26 3 air", "setblock 0 -26 -3 air", "tp @s 0.5 -26 0.5 -90 0", ""])
}

/// Blocks set at 10 -60 0 over the block below them (and with the block above, if any), then broken by
/// `setblock ... destroy`, each so many times; the items cleared after each kind.
const BROKEN: [(&str, &str, &str, usize); 14] = [
    ("pumpkin_stem [\"growth\"=7]", "farmland", "", 6),
    ("pumpkin_stem [\"growth\"=0]", "farmland", "", 4),
    ("melon_stem [\"growth\"=4]", "farmland", "", 4),
    ("pumpkin_stem [\"growth\"=7,\"facing_direction\"=2]", "farmland", "", 2),
    ("composter [\"composter_fill_level\"=8]", "grass_block", "", 2),
    ("composter [\"composter_fill_level\"=3]", "grass_block", "", 1),
    ("pale_moss_carpet [\"upper_block_bit\"=false]", "grass_block", "", 1),
    ("pale_moss_carpet [\"upper_block_bit\"=true]", "grass_block", "", 1),
    ("mangrove_propagule [\"hanging\"=false,\"propagule_stage\"=0]", "grass_block", "", 2),
    ("mangrove_propagule [\"hanging\"=true,\"propagule_stage\"=4]", "air", "mangrove_leaves", 2),
    ("mangrove_propagule [\"hanging\"=true,\"propagule_stage\"=2]", "air", "mangrove_leaves", 2),
    ("pitcher_crop [\"growth\"=4,\"upper_block_bit\"=false]", "farmland", "pitcher_crop [\"growth\"=4,\"upper_block_bit\"=true]", 2),
    ("pitcher_crop [\"growth\"=2,\"upper_block_bit\"=false]", "farmland", "", 2),
    ("pitcher_crop [\"growth\"=1,\"upper_block_bit\"=false]", "farmland", "", 1),
];

fn stem() -> Vec<String> {
    let mut lines: Vec<String> = [
        "kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "time set 1000", "tp @a 0.5 -60 0.5", "fill -8 -61 -8 16 -61 8 grass_block", "fill -8 -60 -8 16 -55 8 air", "",
    ]
    .map(String::from)
    .into();
    for (block, under, over, times) in BROKEN {
        for _ in 0..times {
            lines.extend([format!("setblock 10 -61 0 {under}"), format!("setblock 10 -60 0 {block}")]);
            lines.extend((!over.is_empty()).then(|| format!("setblock 10 -59 0 {over}")));
            lines.extend(["setblock 10 -60 0 air destroy".into(), String::new(), "setblock 10 -59 0 air".into()]);
        }
        lines.push("kill @e[type=item]".into());
    }
    lines.extend(["fill -8 -61 -8 16 -61 8 grass_block".into(), String::new()]);
    lines
}
