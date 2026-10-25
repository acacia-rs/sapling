//! Riding and vehicles on a server, as an operator actor does them while a second operator watches:
//! `/ride` answered, mounts and dismounts by hand, saddles, boats on water and land, and minecarts on
//! rails (docs/research/vehicles.md). Prints what both were answered, as bds_command_probe does.
//! `cargo run -p sapling --example bds_ride_probe -- <server> [ride|mounts|boats|carts]`
#[path = "shared/dump.rs"]
#[allow(dead_code)]
mod dump;
#[path = "shared/probe.rs"]
#[allow(dead_code)]
mod probe;

use std::time::{Duration, Instant};

use acacia_bot::proto::{Packet, packets};
use acacia_bot::{Bot, BotEvent};
use probe::{clock, connect};

const SHOWN: &[&str] = &[
    "SetEntityLink", "SetEntityData", "AddEntity", "AddItemEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityMotion", "MovePlayer",
    "CorrectPlayerMovePrediction", "ContainerOpen", "ContainerClose", "UpdateEquipment", "InventoryContent", "InventorySlot", "EntityEvent", "UpdateAttributes",
    "LevelSoundEvent", "LevelEvent", "Text", "MobEquipment", "MobArmorEquipment", "Animate",
];
const SETTLE: Duration = Duration::from_millis(2000);
/// Characters of a packet shown.
const CHARS: usize = 1600;

fn lines(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|line| line.to_string()).collect()
}

/// `/ride`'s overloads, each answered, beside the links and seats they make.
fn ride() -> Vec<String> {
    lines(&[
        "gamerule domobspawning false",
        "gamerule sendcommandfeedback true",
        "kill @e[type=!player]",
        "summon pig ~2 ~ ~",
        "ride @s start_riding @e[type=pig,c=1]",
        "ride @s start_riding @e[type=pig,c=1]",
        "ride @s stop_riding",
        "ride @s stop_riding",
        "ride @e[type=pig] summon_rider zombie",
        "ride @e[type=pig] summon_rider zombie",
        "ride @e[type=pig] evict_riders",
        "ride @e[type=pig] evict_riders",
        "ride @s summon_ride boat",
        "ride @s summon_ride boat",
        "ride @s summon_ride horse no_ride_change",
        "ride @s summon_ride minecart reassign_rides",
        "ride @s summon_ride minecart skip_riders",
        "ride @s stop_riding",
        "kill @e[type=!player]",
        "summon boat ~2 ~ ~",
        "summon zombie ~3 ~ ~",
        "summon zombie ~3 ~ ~1",
        "summon zombie ~3 ~ ~2",
        "ride @e[type=zombie] start_riding @e[type=boat,c=1] teleport_rider if_group_fits",
        "ride @e[type=zombie] start_riding @e[type=boat,c=1] teleport_ride until_full",
        "ride @e[type=boat] evict_riders",
        "ride @e[type=zombie] start_riding @e[type=boat,c=1] teleport_ride until_full",
        "ride @s start_riding @e[type=boat,c=1]",
        "ride @e[type=boat] evict_riders",
        "ride @s start_riding @e[type=zombie,c=1]",
        "ride @e[type=zombie,c=1] start_riding @s",
        "ride @s summon_rider nosuchmob",
        "ride @e[type=boat] summon_rider pig",
        "kill @e[type=!player]",
    ])
}

/// Mounting and leaving by hand, a saddle put on, a carrot on a stick held, a horse tried.
fn mounts() -> Vec<String> {
    lines(&[
        "gamerule domobspawning false",
        "kill @e[type=!player]",
        "clear @s",
        "summon pig ~2 ~ ~",
        "!mount pig",
        "",
        "!dismount",
        "",
        "replaceitem entity @s slot.weapon.mainhand 0 saddle",
        "!use pig",
        "",
        "replaceitem entity @s slot.weapon.mainhand 0 carrot_on_a_stick",
        "!mount pig",
        "",
        "",
        "",
        "!dismount",
        "kill @e[type=!player]",
        "clear @s",
        "summon horse ~2 ~ ~",
        "!mount horse",
        "",
        "",
        "",
        "",
        "!dismount",
        "kill @e[type=!player]",
        "summon minecart ~2 ~ ~",
        "!mount minecart",
        "",
        "!dismount",
        "kill @e[type=!player]",
    ])
}

/// A boat put on water from the hand, drifting, sat in, and broken; one on land; a chest boat.
fn boats() -> Vec<String> {
    lines(&[
        "gamerule domobspawning false",
        "kill @e[type=!player]",
        "fill ~2 ~-3 ~-3 ~8 ~-1 ~3 water",
        "replaceitem entity @s slot.weapon.mainhand 0 oak_boat",
        "!place 3 -1 0",
        "",
        "",
        "!mount boat",
        "",
        "!dismount",
        "gamemode s",
        "!hit boat",
        "!hit boat",
        "!hit boat",
        "",
        "gamemode c",
        "kill @e[type=!player]",
        "summon boat ~-2 ~ ~",
        "",
        "summon chest_boat ~-2 ~ ~2",
        "summon cow ~-2 ~ ~2",
        "",
        "",
        "replaceitem entity @s slot.weapon.mainhand 0 birch_chest_boat",
        "!place 4 -1 2",
        "replaceitem entity @s slot.weapon.mainhand 0 bamboo_raft",
        "!place 5 -1 -2",
        "",
        "kill @e[type=!player]",
        "fill ~2 ~-3 ~-3 ~8 ~-1 ~3 air",
        "fill ~2 ~-3 ~-3 ~8 ~-3 ~3 stone",
    ])
}

/// Minecarts on rails: one put down, launched off a powered rail, up a slope and round a curve,
/// one with TNT on a lit activator rail, and a hopper cart.
fn carts() -> Vec<String> {
    lines(&[
        "gamerule domobspawning false",
        "kill @e[type=!player]",
        "fill ~2 ~-1 ~-6 ~24 ~3 ~6 air",
        "fill ~2 ~-2 ~-6 ~24 ~-2 ~6 stone",
        "fill ~4 ~-1 ~ ~14 ~-1 ~ rail",
        "setblock ~3 ~-1 ~ stone",
        "setblock ~4 ~-1 ~ golden_rail",
        "summon minecart ~4 ~-1 ~",
        "",
        "setblock ~4 ~-2 ~ redstone_block",
        "",
        "",
        "",
        "kill @e[type=!player]",
        "fill ~4 ~-1 ~ ~14 ~-1 ~ air",
        "setblock ~4 ~-1 ~ golden_rail",
        "setblock ~5 ~-1 ~ golden_rail",
        "setblock ~6 ~-1 ~ rail",
        "setblock ~7 ~-1 ~ stone",
        "setblock ~7 ~ ~ rail",
        "setblock ~8 ~-1 ~ stone",
        "setblock ~8 ~ ~ stone",
        "setblock ~8 ~1 ~ rail",
        "setblock ~9 ~1 ~ air",
        "setblock ~4 ~-2 ~ redstone_block",
        "setblock ~5 ~-2 ~ redstone_block",
        "summon minecart ~6 ~-1 ~",
        "",
        "",
        "kill @e[type=!player]",
        "fill ~2 ~-1 ~-6 ~24 ~3 ~6 air",
        "fill ~2 ~-2 ~-6 ~24 ~-2 ~6 stone",
        "setblock ~4 ~-1 ~ activator_rail",
        "summon tnt_minecart ~4 ~-1 ~",
        "setblock ~4 ~-2 ~ redstone_block",
        "",
        "",
        "",
        "",
        "kill @e[type=!player]",
        "fill ~2 ~-1 ~-6 ~24 ~3 ~6 air",
        "fill ~2 ~-2 ~-6 ~24 ~-2 ~6 stone",
        "setblock ~4 ~-1 ~ rail",
        "summon hopper_minecart ~4 ~-1 ~",
        "summon chest_minecart ~6 ~-1 ~",
        "summon minecart ~8 ~-1 ~",
        "",
        "kill @e[type=!player]",
    ])
}

fn show(start: Instant, who: &str, event: Option<BotEvent>) -> Result<(), String> {
    let packet = match event {
        Some(BotEvent::Packet(packet)) => packet,
        Some(BotEvent::Disconnected(reason)) => return Err(format!("{who} disconnected: {reason:?}")),
        Some(_) => return Ok(()),
        None => return Err(format!("{who} is gone")),
    };
    let ms = clock::tick_ms(start);
    if packet.id == packets::CommandOutput::ID {
        let answer = packet.decode::<packets::CommandOutput>().map_err(|e| e.to_string())?;
        let lines: Vec<String> = answer.output.iter().map(|line| format!("{} success={} {:?}", line.message_id, line.success, line.parameters)).collect();
        println!("{ms:>7} {who:<7} CommandOutput success_count={} {}", answer.success_count, lines.join(" | "));
    } else if let Some(name) = acacia_bot::proto::packet_name(packet.id).filter(|name| SHOWN.contains(name)) {
        let text: String = dump::dump(&packet).unwrap_or_default().chars().take(CHARS).collect();
        println!("{ms:>7} {who:<7} {name} {text}");
    }
    Ok(())
}

/// The newest entity of `kind` (no namespace) the actor knows of.
fn newest(actor: &Bot, kind: &str) -> Option<u64> {
    let kind = format!("minecraft:{kind}");
    actor.state().entities.iter().filter(|entity| entity.kind == kind).map(|entity| entity.runtime_id).max()
}

/// The block `offset` (three whole numbers) from the actor's feet.
fn from_feet(actor: &Bot, offset: &str) -> Option<[i32; 3]> {
    let offset: Vec<i32> = offset.split(' ').filter_map(|part| part.parse().ok()).collect();
    let feet = actor.movement().and_then(|movement| movement.position()).map(|at| at.map(|c| c.floor() as i32))?;
    let &[x, y, z] = &offset[..] else { return None };
    Some([feet[0] + x, feet[1] + y, feet[2] + z])
}

/// `!mount kind`, `!use kind` (the held item on it), `!hit kind`, `!dismount`, `!place x y z` (the
/// held item on the top of the block that far from the feet).
async fn act(actor: &mut Bot, line: &str) {
    let (verb, rest) = line.split_once(' ').unwrap_or((line, ""));
    let id = newest(actor, rest.trim());
    let done = match (verb, id) {
        ("!mount", Some(id)) => format!("{:?}", actor.mount(id).await.map(|_| ())),
        ("!use", Some(id)) => format!("{:?}", actor.interact_entity(id).await),
        ("!hit", Some(id)) => format!("{:?}", actor.attack(id).await),
        ("!dismount", _) => format!("{:?}", actor.dismount().await),
        ("!place", _) => match from_feet(actor, rest) {
            Some(at) => format!("{:?}", actor.use_item_on_block(at, acacia_bot::interact::Face::Up).await),
            None => "no feet".into(),
        },
        _ => format!("nothing to {verb}"),
    };
    println!("## {line}: {done}");
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let server = args.next().ok_or("usage: bds_ride_probe <server> [ride|mounts|boats|carts]")?;
    let lines = match args.next().as_deref() {
        None | Some("ride") => ride(),
        Some("mounts") => mounts(),
        Some("boats") => boats(),
        Some("carts") => carts(),
        Some(other) => return Err(format!("no drill {other}").into()),
    };
    let (mut actor, mut watcher) = (connect(&server, "RideActor").await?, connect(&server, "RideWatcher").await?);
    watcher.wait_ticks(60).await?;
    actor.wait_ticks(60).await?;
    for bot in [&mut actor, &mut watcher] {
        while let Ok(Some(_)) = tokio::time::timeout(Duration::from_millis(5), bot.next()).await {}
    }
    let start = Instant::now();
    clock::mark_start();
    for line in &lines {
        println!("## /{line}");
        if line.starts_with('!') {
            let action = act(&mut actor, line);
            tokio::pin!(action);
            loop {
                tokio::select! {
                    _ = &mut action => break,
                    event = watcher.next() => show(start, "watcher", event)?,
                }
            }
        } else if !line.is_empty() {
            let _ = actor.client().command(line);
        }
        let end = tokio::time::Instant::now() + clock::scaled(SETTLE);
        loop {
            tokio::select! {
                event = actor.next() => show(start, "actor", event)?,
                event = watcher.next() => show(start, "watcher", event)?,
                _ = tokio::time::sleep_until(end) => break,
            }
        }
    }
    actor.disconnect().await;
    watcher.disconnect().await;
    Ok(())
}
