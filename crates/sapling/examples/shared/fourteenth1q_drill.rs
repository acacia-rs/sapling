//! The fourteenth wave's group Q drills (research/parity/FOURTEENTH-BRIEF.md). On the flat ground at
//! midnight: `q1warden`, a held warden 10 off a warded survival actor, fresh for each disturbance (a pig
//! hurt three times, a chest opened and shut, a snowball thrown, the actor hurt, a door opened, a block
//! placed); `q1ghast`, a held ghast's fireballs ending on the actor, a wall at its back, from ahead and
//! from aside; `q1ender`, four calm endermen at night; `q1ender2`, four at noon under the sky, then four
//! under a glowstone roof over a glowstone floor. On creature_drill.rs's floor: `q1meal`, a creative
//! actor clicking use on foods and drinks in set rhythms (`!q1air`); `q1piglin`, piglins holding or
//! wearing one thing offered another.

use std::time::Duration;

use acacia_bot::Bot;
use acacia_bot::interact::to_wire;
use acacia_bot::proto::packets::InventoryTransaction;
use acacia_bot::proto::types::{
    BlockCoordinates, Transaction, TransactionLegacy, TransactionTransactionData, TransactionTransactionType, TransactionUseItem, TransactionUseItemActionType,
    TransactionUseItemClientCooldownState, TransactionUseItemClientPrediction, TransactionUseItemHand, TransactionUseItemTriggerType, Vec3f,
};
use acacia_bot::BotEvent;

use super::creatures::{floor, waits, with};

const WARDEN_SHOWN: &[&str] = &["AddEntity", "SetEntityData", "EntityEvent", "LevelSoundEvent", "LevelEvent", "CommandOutput", "UpdateBlock", "MobEffect", "RemoveEntity"];
const GHAST_SHOWN: &[&str] = &[
    "AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityMotion", "SetEntityData", "EntityEvent", "LevelEvent", "LevelSoundEvent", "MovePlayer", "CommandOutput",
];
const ENDER_SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "LevelEvent", "LevelSoundEvent", "CommandOutput"];
const MEAL_SHOWN: &[&str] = &["UpdateAttributes", "SetEntityData", "EntityEvent", "LevelSoundEvent", "CompletedUsingItem", "InventoryContent", "InventorySlot", "CommandOutput"];
const PIGLIN_SHOWN: &[&str] = &[
    "AddEntity", "AddItemEntity", "RemoveEntity", "TakeItemEntity", "MobEquipment", "MobArmorEquipment", "SetEntityData", "LevelSoundEvent", "EntityEvent", "CommandOutput",
];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "q1warden" => (warden(), WARDEN_SHOWN),
        "q1ghast" => (ghast(), GHAST_SHOWN),
        "q1ender" => (enders_night(), ENDER_SHOWN),
        "q1ender2" => (enders_day(), ENDER_SHOWN),
        "q1meal" => (meals(), MEAL_SHOWN),
        "q1piglin" => (piglins(), PIGLIN_SHOWN),
        _ => return None,
    })
}

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

/// The flat ground cleared about the spawn at `time`, both players creative, the watcher far south.
fn ground(time: u32) -> Vec<String> {
    let mut out = owned(["kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "gamerule falldamage false", "gamerule mobgriefing false", "weather clear", "difficulty easy"]);
    out.extend([format!("time set {time}"), "gamemode creative @a".into(), "effect @a clear".into(), "clear @s".into(), "fill -12 -60 -12 12 -50 16 air".into(), "tp CmdWatcher 0.5 -60 -14.5".into(), "tp @s 0.5 -60 0.5 0 0".into(), String::new()]);
    out
}

fn end() -> Vec<String> {
    owned(["kill @e[type=!player]", "gamemode creative @s", "effect @s clear", "clear @s", "gamerule mobgriefing true", "tp @s 0.5 -60 0.5 0 0", ""])
}

/// Each disturbance of the warded survival actor at the spawn, heard by a fresh warden held 10 off.
fn warden() -> Vec<String> {
    let mut out = ground(18000);
    out.extend(owned(["gamemode survival @s", "effect @s resistance 1000 255 true", "effect @s regeneration 1000 255 true", "setblock 2 -60 0 chest", "setblock -2 -60 0 wooden_door", "tp @s 0.5 -60 0.5 0 0"]));
    let fresh = ["kill @e[type=warden]", "summon warden 0.5 -60 10.5", "effect @e[type=warden] slowness 1000 255 true"];
    out.extend(owned(["summon pig 0.5 -60 5.5", "effect @e[type=pig] slowness 1000 255 true"]));
    out.extend(owned(fresh));
    waits(&mut out, 3);
    for _ in 0..3 {
        out.extend(owned(["damage @e[type=pig] 1"]));
        waits(&mut out, 3);
    }
    waits(&mut out, 6);
    out.extend(owned(["kill @e[type=pig]"]));
    let parts: [&[&str]; 5] = [
        &["!click 2 0 0", "", "", "!close"],
        &["replaceitem entity @s slot.weapon.mainhand 0 snowball 4", "!look -3 4 -3", "", "!use"],
        &["damage @s 1"],
        &["!click -2 0 0 east"],
        &["replaceitem entity @s slot.weapon.mainhand 0 stone 4", "!place 1 -1 -1"],
    ];
    for part in parts {
        out.extend(owned(fresh));
        waits(&mut out, 3);
        out.extend(part.iter().map(|line| line.to_string()));
        waits(&mut out, 4);
    }
    out.extend(owned(["setblock 2 -60 0 air", "setblock -2 -60 0 air", "setblock -2 -59 0 air", "fill -1 -60 -1 1 -60 -1 air"]));
    out.extend(end());
    out
}

/// A held ghast's shots end on the warded survival actor, a stone wall at its back: three from
/// ahead, three from aside.
fn ghast() -> Vec<String> {
    let mut out = ground(18000);
    out.extend(owned(["fill -2 -60 -1 2 -58 -1 stone", "gamemode survival @s", "effect @s resistance 1000 255 true", "effect @s fire_resistance 1000 0 true", "tp @s 0.5 -60 0.5 0 0"]));
    out.extend(owned(["summon ghast 0.5 -59 14.5", "effect @e[type=ghast] slowness 1000 255 true"]));
    for at in ["0.5 -59 14.5", "9.5 -59 11.5"] {
        out.push(format!("tp @e[type=ghast] {at}"));
        for _ in 0..3 {
            waits(&mut out, 4);
            out.push("tp @s 0.5 -60 0.5 0 0".into());
        }
    }
    waits(&mut out, 2);
    out.extend(owned(["fill -2 -60 -1 2 -58 -1 air"]));
    out.extend(end());
    out
}

const ENDERS: [&str; 4] = ["summon enderman 8.5 -60 8.5", "summon enderman -7.5 -60 8.5", "summon enderman 8.5 -60 -7.5", "summon enderman -7.5 -60 -7.5"];

/// Four calm endermen left three minutes at midnight under the open sky.
fn enders_night() -> Vec<String> {
    let mut out = ground(18000);
    out.extend(owned(ENDERS));
    waits(&mut out, 180);
    out.extend(end());
    out
}

/// Four calm endermen at noon under the open sky for 80 s; then four under a glowstone roof (y -56)
/// over a glowstone floor (y -61), 49 across, for two minutes.
fn enders_day() -> Vec<String> {
    let mut out = ground(6000);
    out.extend(owned(ENDERS));
    waits(&mut out, 80);
    out.extend(owned(["kill @e[type=enderman]", "fill -24 -56 -24 24 -56 24 glowstone", "fill -24 -61 -24 24 -61 24 glowstone", ""]));
    out.extend(owned(ENDERS));
    waits(&mut out, 120);
    out.extend(owned(["kill @e[type=enderman]", "fill -24 -56 -24 24 -56 24 air", "fill -24 -61 -24 24 -61 24 grass_block"]));
    out.extend(end());
    out
}

fn hold(item: &str) -> String {
    format!("replaceitem entity @s slot.weapon.mainhand 0 {item}")
}

/// For each food and drink, the creative actor clicks use once; again long after; a finishing click
/// (the simulation tick's) just after the first meal ended; that and a fresh click; and clicks every 4
/// ticks for 5 s, as a button held down.
fn meals() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher 0.5 -26 7.5"]);
    let held: String = (0..25).map(|i| format!(" p{}", i * 4)).collect();
    let rhythms = ["!q1air p0".to_string(), "!q1air p0 p40".into(), "!q1air p0 f33".into(), "!q1air p0 f33 p37".into(), format!("!q1air{held}")];
    for item in ["bread 2", "golden_apple 2", "potion 1 0", "honey_bottle 2"] {
        lines = with(lines, &[&hold(item), ""]);
        for rhythm in &rhythms {
            lines = with(lines, &[rhythm, "", "", ""]);
        }
    }
    with(lines, &["clear @s", ""])
}

/// For each case a piglin 3 east of the creative actor, its hand and head set, and an item dropped
/// towards it; killed 6 s on, so what it holds and wears falls.
fn piglins() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher 0.5 -26 7.5"]);
    let cases = [
        ("iron_sword", "air", "crossbow"),
        ("crossbow", "air", "iron_sword"),
        ("golden_sword", "air", "crossbow"),
        ("air", "air", "iron_sword"),
        ("air", "air", "iron_helmet"),
        ("air", "iron_helmet", "diamond_helmet"),
        ("air", "diamond_helmet", "iron_helmet"),
        ("air", "air", "leather_boots"),
    ];
    for (main, head, offered) in cases {
        lines = with(lines, &["kill @e[type=item]", "summon piglin 3.5 -26 0.5 90 0 minecraft:spawn_adult"]);
        for slot in ["slot.armor.chest", "slot.armor.legs", "slot.armor.feet", "slot.weapon.offhand"] {
            lines.push(format!("replaceitem entity @e[type=piglin] {slot} 0 air"));
        }
        lines.push(format!("replaceitem entity @e[type=piglin] slot.weapon.mainhand 0 {main}"));
        lines.push(format!("replaceitem entity @e[type=piglin] slot.armor.head 0 {head}"));
        lines = with(lines, &[&format!("replaceitem entity @s slot.hotbar 0 {offered} 1"), "", "!drop 0 1"]);
        waits(&mut lines, 6);
        lines = with(lines, &["kill @e[type=piglin]", "", ""]);
    }
    with(lines, &["kill @e[type=!player]", "clear @s", ""])
}

fn click_air(bot: &Bot, finish: bool) -> InventoryTransaction {
    let inventory = &bot.state().inventory;
    let [x, y, z] = bot.eye_position();
    let (trigger_type, client_prediction) = match finish {
        true => (TransactionUseItemTriggerType::SimulationTick, TransactionUseItemClientPrediction::Success),
        false => (TransactionUseItemTriggerType::UnknownValue, TransactionUseItemClientPrediction::Failure),
    };
    InventoryTransaction {
        transaction: Transaction {
            legacy: TransactionLegacy { legacy_request_id: 0, legacy_transactions: None },
            transaction_type: TransactionTransactionType::ItemUse,
            actions: Vec::new(),
            transaction_data: TransactionTransactionData::ItemUse(TransactionUseItem {
                action_type: TransactionUseItemActionType::ClickAir,
                trigger_type,
                block_position: BlockCoordinates { x: 0, y: 0, z: 0 },
                face: 255,
                hotbar_slot: i32::from(inventory.selected_hotbar_slot),
                hand: TransactionUseItemHand::MainHand,
                held_item: to_wire(inventory.held()),
                player_pos: Vec3f { x, y, z },
                click_pos: Vec3f { x: 0.0, y: 0.0, z: 0.0 },
                block_runtime_id: 0,
                client_prediction,
                client_cooldown_state: TransactionUseItemClientCooldownState::Off,
            }),
        },
    }
}

/// The actor's own ticks for `ticks`, with what it is told of its meals printed by the tick it came in.
async fn run(actor: &mut Bot, ticks: u32, from: u32) {
    for tick in from..from + ticks {
        let end = tokio::time::Instant::now() + crate::probe::clock::scaled(Duration::from_millis(50));
        while let Ok(Some(event)) = tokio::time::timeout_at(end, actor.next()).await {
            if let BotEvent::Packet(packet) = event {
                let name = acacia_bot::proto::packet_name(packet.id).unwrap_or("?");
                if ["CompletedUsingItem", "UpdateAttributes", "InventoryContent"].contains(&name) {
                    let text: String = crate::dump::dump(&packet).unwrap_or_default().chars().take(90).collect();
                    println!("## q1 tick {tick} actor {name} {text}");
                }
            }
        }
        tokio::time::sleep_until(end).await;
    }
}

/// Does `!q1air p0 f33 p37 ...`: a plain click (`p`, the press's) or a finishing one (`f`, the
/// simulation tick's) in the air at each tick count, the actor's ticks run between.
pub async fn act(actor: &mut Bot, line: &str) -> bool {
    let Some(words) = line.strip_prefix("!q1air ") else { return false };
    let mut tick = 0;
    for word in words.split(' ') {
        let (finish, at) = (word.starts_with('f'), word[1..].parse::<u32>().unwrap_or(tick));
        run(actor, at.saturating_sub(tick), tick).await;
        tick = tick.max(at);
        actor.client().send(&click_air(actor, finish));
        println!("## q1 tick {tick} clicked {}", if finish { "finish" } else { "plain" });
    }
    run(actor, 40, tick).await;
    true
}
