//! Runs a drill of command lines as an operator bot while a second operator watches, and prints
//! what each was answered: CommandOutput in full, Text, GameRulesChanged as `rule=value`, and the
//! drill's own packets. The ground truth for sapling's vanilla commands (`sapling::vanilla`).
//! `cargo run -p sapling --example bds_command_probe -- <server> [gamerule|kill|replaceitem]` (offline mode,
//! operators by default; `gamerule` when none is named).
#[path = "shared/dump.rs"]
#[allow(dead_code)]
mod dump;
#[path = "shared/probe.rs"]
#[allow(dead_code)]
mod probe;

use std::time::{Duration, Instant};

use acacia_bot::BotEvent;
use acacia_bot::proto::{Packet, packets};
use probe::{SETTLE, connect};

const GAMERULE: &[&str] = &[
    "gamerule",
    "gamerule dodaylightcycle",
    "gamerule dodaylightcycle false",
    "gamerule dodaylightcycle false",
    "gamerule DoDaylightCycle true",
    "gamerule randomtickspeed",
    "gamerule randomtickspeed 5",
    "gamerule randomtickspeed true",
    "gamerule randomtickspeed -3",
    "gamerule dodaylightcycle 3",
    "gamerule nosuchrule true",
    "gamerule spawnradius 2000000000",
    "gamerule playerssleepingpercentage 150",
    "gamerule playerssleepingpercentage -1",
    "gamerule playerssleepingpercentage 100",
    "gamerule randomtickspeed 1",
    "gamerule randomtickspeed 4097",
    "gamerule spawnradius 128",
    "gamerule spawnradius -1",
    "gamerule functioncommandlimit 0",
    "gamerule functioncommandlimit 10001",
    "gamerule maxcommandchainlength -1",
    "gamerule DoDaylightCycle",
    "gamerule playerwaypoints",
    "gamerule playerwaypoints banana",
    "gamerule playerwaypoints off",
    "gamerule playerwaypoints 0",
    "gamerule playerwaypoints everyone",
    "gamerule locatorbar",
    "gamerule locatorbar false",
    "gamerule locatorbar true",
    "gamerule dodaylightcycle true extra",
    "gamerule",
];

/// Mobs killed: the death, the loot and when they go; with `domobloot` off, no loot.
const KILL: &[&str] = &[
    "gamerule domobspawning false",
    "summon chicken ~2 ~ ~",
    "kill @e[type=chicken]",
    "summon pig ~-2 ~ ~",
    "kill @e[type=pig]",
    "summon armor_stand ~ ~ ~2",
    "kill @e[type=armor_stand]",
    "gamerule domobloot false",
    "summon chicken ~2 ~ ~",
    "kill @e[type=chicken]",
    "gamerule domobloot true",
];
/// Items put into a chest's slots, and the ways that fails.
const REPLACEITEM: &[&str] = &[
    "setblock ~2 ~ ~ chest",
    "replaceitem block ~2 ~ ~ slot.container 0 diamond 10",
    "replaceitem block ~2 ~ ~ slot.container 1 diamond",
    "replaceitem block ~2 ~ ~ slot.container 2 stone 70",
    "replaceitem block ~2 ~ ~ slot.container 27 diamond",
    "replaceitem block ~2 ~ ~ slot.container -1 diamond",
    "replaceitem block ~2 ~ ~ slot.container 3 no_such_item",
    "replaceitem block ~2 ~ ~ slot.container 0 air",
    "replaceitem block ~ ~-1 ~ slot.container 0 diamond",
    "replaceitem block ~2 ~ ~ slot.inventory 0 diamond",
];
const REPLACEITEM_SHOWN: &[&str] = &["InventorySlot", "InventoryContent", "BlockEntityData"];

const KILL_SHOWN: &[&str] = &["EntityEvent", "AddEntity", "AddItemEntity", "RemoveEntity", "LevelSoundEvent", "SetEntityData", "Animate", "MobEffect"];
/// How long each line of the kill drill is watched: a dead mob goes a second after.
const KILL_SETTLE: Duration = Duration::from_millis(2500);

fn show(start: Instant, shown: &[&str], who: &str, event: Option<BotEvent>) -> Result<(), String> {
    let packet = match event {
        Some(BotEvent::Packet(packet)) => packet,
        Some(BotEvent::Disconnected(reason)) => return Err(format!("{who} disconnected: {reason:?}")),
        Some(_) => return Ok(()),
        None => return Err(format!("{who} is gone")),
    };
    let ms = start.elapsed().as_millis();
    if packet.id == packets::CommandOutput::ID {
        let answer = packet.decode::<packets::CommandOutput>().map_err(|e| e.to_string())?;
        let lines: Vec<String> = answer.output.iter().map(|line| format!("{} success={} {:?}", line.message_id, line.success, line.parameters)).collect();
        println!("{ms:>7} {who:<7} CommandOutput success_count={} data={:?} {}", answer.success_count, answer.data, lines.join(" | "));
    } else if packet.id == packets::GameRulesChanged::ID {
        let rules = packet.decode::<packets::GameRulesChanged>().map_err(|e| e.to_string())?;
        let shown: Vec<String> = rules.rules.iter().map(|rule| format!("{}={:?}", rule.name, rule.value)).collect();
        println!("{ms:>7} {who:<7} GameRulesChanged {}", shown.join(" "));
    } else if let Some(name) = acacia_bot::proto::packet_name(packet.id).filter(|name| ["Text", "SetTime", "SyncWorldClocks"].contains(name) || shown.contains(name)) {
        let text: String = dump::dump(&packet).unwrap_or_default().chars().take(SHOWN).collect();
        println!("{ms:>7} {who:<7} {name} {text}");
    }
    Ok(())
}

/// Characters of a drill's own packets shown.
const SHOWN: usize = 900;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let server = args.next().ok_or("usage: bds_command_probe <server> [gamerule|kill|replaceitem]")?;
    let (lines, shown, settle) = match args.next().as_deref() {
        None | Some("gamerule") => (GAMERULE, &[][..], SETTLE),
        Some("kill") => (KILL, KILL_SHOWN, KILL_SETTLE),
        Some("replaceitem") => (REPLACEITEM, REPLACEITEM_SHOWN, SETTLE),
        Some(other) => return Err(format!("no drill {other}").into()),
    };
    let (mut actor, mut watcher) = (connect(&server, "CmdActor").await?, connect(&server, "CmdWatcher").await?);
    watcher.wait_ticks(60).await?;
    actor.wait_ticks(60).await?;
    for bot in [&mut actor, &mut watcher] {
        while let Ok(Some(_)) = tokio::time::timeout(Duration::from_millis(5), bot.next()).await {}
    }
    let start = Instant::now();
    for line in lines {
        println!("## /{line}");
        actor.client().command(line);
        let end = tokio::time::Instant::now() + settle;
        loop {
            tokio::select! {
                event = actor.next() => show(start, shown, "actor", event)?,
                event = watcher.next() => show(start, shown, "watcher", event)?,
                _ = tokio::time::sleep_until(end) => break,
            }
        }
    }
    actor.disconnect().await;
    watcher.disconnect().await;
    Ok(())
}
