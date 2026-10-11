//! Records what a vanilla server sends for one of every summonable kind, grown and, where the
//! kind has one, baby, and for a dropped item: the source of `Mobs.bin` and `ItemEntity.bin` in
//! `sapling-data/data` (that crate's README). It starts the server itself to type the summon
//! commands into its console, and stops it when done.
//! `cargo run -p sapling --example bds_mob_recorder -- <BDS folder> <its address> crates/sapling-data/data`
//! (an offline-mode server; its world is left with the rules and difficulty set here)
use std::io::Write;
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::time::{Duration, Instant};

use acacia_bot::client::{Client, PacketFilter};
use acacia_bot::proto::Packet;
use acacia_bot::proto::nbt::Value;
use acacia_bot::proto::packets::{AddEntity, AddItemEntity, AvailableEntityIdentifiers, SetSpawnPosition};
use acacia_bot::proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Metadata, MetadataFlags1};
use acacia_bot::{Bot, BotConfig, BotEvent};
use bytes::Bytes;

const PATIENCE: Duration = Duration::from_secs(120);
/// How long a summoned kind has to show up; some are not entities a client is told of.
const SHOWS_WITHIN: Duration = Duration::from_millis(1500);
/// So that nothing spawns by itself, hunts the bot, burns or blows up while it is recorded.
const SETUP: &[&str] = &[
    "gamemode creative @a",
    "difficulty normal",
    "time set midnight",
    "gamerule domobspawning false",
    "gamerule domobloot false",
    "gamerule mobgriefing false",
    "gamerule tntexplodes false",
    "gamerule dodaylightcycle false",
    "kill @e[type=!player]",
];
const GROWN_TRIES: usize = 8;
const BABY_EVENT: &str = "minecraft:entity_born";
const PLAYER: &str = "minecraft:player";

type Failure = Box<dyn std::error::Error>;

async fn connect(server: &str) -> Result<Bot, Failure> {
    let deadline = Instant::now() + PATIENCE;
    let subscribe = PacketFilter::none().with(AddEntity::ID).with(AddItemEntity::ID).with(AvailableEntityIdentifiers::ID).with(SetSpawnPosition::ID);
    loop {
        let config = BotConfig { physics: true, subscribe: subscribe.clone(), ..BotConfig::default() };
        match Bot::connect(Client::builder(server).offline("MobRecorder").chunk_radius(4), config).await {
            Ok(bot) => return Ok(bot),
            Err(error) if Instant::now() > deadline => return Err(error.into()),
            Err(_) => tokio::time::sleep(Duration::from_secs(2)).await,
        }
    }
}

fn summonable(identifiers: &AvailableEntityIdentifiers) -> Vec<String> {
    let Ok(nbt) = identifiers.nbt.decode() else { return Vec::new() };
    let Some(Value::List(list)) = nbt.value.get("idlist") else { return Vec::new() };
    let kind = |entry: &Value| match (entry.get("id"), entry.get("summonable")) {
        (Some(Value::String(id)), Some(Value::Byte(1))) if id != PLAYER => Some(id.to_string()),
        _ => None,
    };
    list.items.iter().filter_map(kind).collect()
}

/// The packets of `id` that arrive within `wait`.
async fn packets(bot: &mut Bot, id: u32, wait: Duration) -> Vec<acacia_bot::proto::RawPacket> {
    let end = tokio::time::Instant::now() + wait;
    let mut found = Vec::new();
    while let Ok(Some(event)) = tokio::time::timeout_at(end, bot.next()).await {
        match event {
            BotEvent::Packet(packet) if packet.id == id => found.push(packet),
            _ => {}
        }
    }
    found
}

/// Summons `kind` beside the bot and returns the AddEntity it is announced with: under its
/// own name or, a villager and a zombie villager, that with `_v2` (bds_kind_recorder.rs added
/// the two this once left out).
async fn summon(console: &mut ChildStdin, bot: &mut Bot, kind: &str, event: &str) -> Result<Option<(AddEntity, Bytes)>, Failure> {
    let event = if event.is_empty() { String::new() } else { format!(" 0 0 {event}") };
    writeln!(console, "execute at @a run summon {kind} ~3 ~ ~{event}")?;
    let end = tokio::time::Instant::now() + SHOWS_WITHIN;
    let mut shown = None;
    while shown.is_none() {
        let Ok(Some(event)) = tokio::time::timeout_at(end, bot.next()).await else { break };
        let BotEvent::Packet(packet) = event else { continue };
        if packet.id == AddEntity::ID {
            shown = packet.decode::<AddEntity>().ok().filter(|add| add.entity_type == kind || add.entity_type == format!("{kind}_v2")).map(|add| (add, packet.body.clone()));
        }
    }
    writeln!(console, "kill @e[type=!player]")?;
    Ok(shown)
}

fn is_baby(add: &AddEntity) -> bool {
    add.metadata.iter().any(|item| item.key == Key::Flags && matches!(&item.value, Metadata::Flags(flags) if flags.contains(MetadataFlags1::BABY)))
}

async fn record(console: &mut ChildStdin, server: &str, out: &Path) -> Result<(), Failure> {
    let mut bot = connect(server).await?;
    let lists = packets(&mut bot, AvailableEntityIdentifiers::ID, Duration::from_secs(3)).await;
    let kinds = summonable(&lists.first().ok_or("no AvailableEntityIdentifiers")?.decode()?);
    for command in SETUP {
        writeln!(console, "{command}")?;
    }
    packets(&mut bot, AddEntity::ID, Duration::from_secs(2)).await;

    let (mut file, mut unseen, mut babies) = (Vec::new(), Vec::new(), 0);
    let mut keep = |body: &Bytes| {
        file.extend((body.len() as u32).to_le_bytes());
        file.extend_from_slice(body);
    };
    for kind in &kinds {
        // Chickens, zombies and their like are a baby now and then without being asked; a
        // tadpole always is, and is kept as that.
        let mut usual = None;
        for _ in 0..GROWN_TRIES {
            usual = summon(console, &mut bot, kind, "").await?;
            if usual.as_ref().is_none_or(|(add, _)| !is_baby(add)) {
                break;
            }
        }
        let Some((usual, body)) = usual else {
            unseen.push(kind.as_str());
            continue;
        };
        keep(&body);
        if is_baby(&usual) {
            continue;
        }
        if let Some((_, body)) = summon(console, &mut bot, kind, BABY_EVENT).await?.filter(|(add, _)| is_baby(add)) {
            keep(&body);
            babies += 1;
        }
    }
    std::fs::write(out.join("Mobs.bin"), &file)?;
    println!("{} of {} kinds recorded, {babies} of them as a baby too ({} bytes)", kinds.len() - unseen.len(), kinds.len(), file.len());
    println!("not announced by AddEntity: {unseen:?}");

    writeln!(console, "execute at @a run loot spawn ~3 ~ ~ loot \"entities/cow\"")?;
    let dropped = packets(&mut bot, AddItemEntity::ID, Duration::from_secs(2)).await;
    let item = dropped.first().ok_or("the loot was not announced by AddItemEntity")?;
    std::fs::write(out.join("ItemEntity.bin"), &item.body)?;
    println!("an item entity: {:?}", item.decode::<AddItemEntity>()?);

    writeln!(console, "setworldspawn 10 70 10")?;
    for packet in packets(&mut bot, SetSpawnPosition::ID, Duration::from_secs(2)).await {
        println!("after setworldspawn 10 70 10: {:?}", packet.decode::<SetSpawnPosition>()?);
    }
    bot.disconnect().await;
    Ok(())
}

fn start(folder: &Path) -> Result<Child, Failure> {
    let log = std::fs::File::create(folder.join("recorder.log"))?;
    Ok(Command::new(folder.join("bedrock_server.exe")).current_dir(folder).stdin(Stdio::piped()).stdout(log).spawn()?)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Failure> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [folder, server, out] = &args[..] else { return Err("usage: bds_mob_recorder <BDS folder> <its address> <sapling-data/data>".into()) };
    let mut bds = start(Path::new(folder))?;
    let mut console = bds.stdin.take().expect("piped");
    let result = record(&mut console, server, Path::new(out)).await;
    // Typed rather than killed: BDS closes its world on `stop`.
    let stopped = writeln!(console, "stop");
    drop(console);
    if stopped.is_err() {
        bds.kill()?;
    }
    bds.wait()?;
    result
}
