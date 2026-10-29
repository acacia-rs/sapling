//! Shows what a vanilla server does with seeded loot it has not rolled yet, on tools/bds-loot-setup.sh's
//! `loot:row` structure (tools/loot_structures.py): the slots a chest of `simple_dungeon.json` seed
//! 357601146 fills on first open; whether a hopper under one or a comparator beside one unpacks it;
//! what a decorated pot with loot drops when broken in survival and in creative; what brushing suspicious sand and gravel does
//! (states, counts, the item). The ground truth for sapling's loot (docs/research/structure-block-entities.md).
//! `cargo run -p sapling --example bds_loot_probe -- <server> [brush] > probe.txt` (offline, operators;
//! `brush`: only the brushing, held as a client holds it).
#[path = "shared/brush_hold.rs"]
mod brush_hold;

use std::time::{Duration, Instant};

use acacia_bot::client::{Client, PacketFilter};
use acacia_bot::interact::Face;
use acacia_bot::proto::{Packet, RawPacket, packets};
use acacia_bot::{Bot, BotConfig, BotEvent};

/// Characters of a decoded packet shown; item tags are long.
const SHOWN: usize = 4000;
const SETTLE: Duration = Duration::from_millis(1500);
const TICK: Duration = Duration::from_millis(50);

macro_rules! shown {
    ($($t:ident),* $(,)?) => {
        /// The packets worth printing, decoded; `None` for the rest.
        fn decoded(p: &RawPacket) -> Option<String> {
            $(if p.id == <packets::$t>::ID {
                return Some(match p.decode::<packets::$t>() {
                    Ok(v) => format!("{} {v:?}", stringify!($t)),
                    Err(e) => format!("{} <decode error: {e}>", stringify!($t)),
                });
            })*
            None
        }
    };
}
shown!(
    InventorySlot, InventoryContent, ItemStackResponse, ContainerOpen, ContainerClose, BlockEvent, LevelSoundEvent, AddItemEntity, UpdateBlock,
    BlockEntityData, LevelEvent, Text, Disconnect,
);

struct Probe {
    start: Instant,
    bot: Bot,
}

impl Probe {
    fn show(&self, event: Option<BotEvent>) -> Result<(), String> {
        match event {
            Some(BotEvent::Packet(packet)) => {
                if let Some(text) = decoded(&packet) {
                    let text: String = text.chars().take(SHOWN).collect();
                    println!("{:>7} {text}", self.start.elapsed().as_millis());
                }
                Ok(())
            }
            Some(BotEvent::Disconnected(reason)) => Err(format!("disconnected: {reason:?}")),
            Some(_) => Ok(()),
            None => Err("the bot is gone".into()),
        }
    }

    async fn settle(&mut self, wait: Duration) -> Result<(), String> {
        let end = tokio::time::Instant::now() + wait;
        loop {
            tokio::select! {
                event = self.bot.next() => self.show(event)?,
                _ = tokio::time::sleep_until(end) => return Ok(()),
            }
        }
    }

    async fn command(&mut self, line: &str) -> Result<(), String> {
        println!("## /{line}");
        self.bot.client().command(line);
        self.settle(SETTLE).await
    }

    async fn act<T: std::fmt::Debug, E: std::fmt::Debug>(&mut self, label: &str, result: Result<T, E>) -> Result<(), String> {
        println!("## {label} -> {result:?}");
        self.settle(SETTLE).await
    }

    async fn open(&mut self, label: &str, pos: [i32; 3]) -> Result<(), String> {
        let opened = self.bot.open_container_at(pos).await.map(|c| c.slots.len());
        self.act(label, opened).await?;
        let closed = self.bot.close_container().await;
        self.act("close", closed).await
    }

    /// Holds use on the block's top for `ticks` ticks, then lets go and waits.
    async fn brush(&mut self, label: &str, pos: [i32; 3], ticks: u32) -> Result<(), String> {
        println!("## brush {label} for {ticks} ticks");
        let top = [pos[0] as f32 + 0.5, pos[1] as f32 + 1.0, pos[2] as f32 + 0.5];
        println!("## look -> {:?}", self.bot.look_at(top).await);
        brush_hold::press(&self.bot, pos, Face::Up);
        for _ in 0..ticks {
            self.settle(TICK).await?;
            brush_hold::held(&self.bot, pos, Face::Up);
        }
        brush_hold::release(&self.bot, pos);
        println!("## let go");
        self.settle(SETTLE).await
    }
}

fn place(pos: [i32; 3]) -> String {
    format!("{} {} {}", pos[0], pos[1], pos[2])
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = std::env::args().nth(1).ok_or("usage: bds_loot_probe <server> [brush]")?;
    let brush_only = std::env::args().nth(2).as_deref() == Some("brush");
    // Mouse input: BDS ignores a creative break from touch input.
    let config = BotConfig { physics: true, subscribe: PacketFilter::all(), mouse_input: true, ..BotConfig::default() };
    let mut bot = Bot::connect(Client::builder(&server).offline("Prober"), config).await?;
    bot.wait_ticks(60).await?;
    for line in ["gamemode creative", "clear @s", "fill ~-8 ~-1 ~-8 ~12 ~-1 ~8 stone", "fill ~-8 ~ ~-8 ~12 ~4 ~8 air"] {
        bot.client().command(line);
        bot.wait_ticks(20).await?;
    }
    while let Ok(Some(_)) = tokio::time::timeout(Duration::from_millis(5), bot.next()).await {}
    let feet = bot.movement().and_then(|m| m.position()).ok_or("the bot has no position")?.map(|c| c.floor() as i32);
    println!("## feet {feet:?}");
    let mut p = Probe { start: Instant::now(), bot };
    // The row lies along +x two blocks south of the prober, on the floor; cell k is at(k).
    let origin = [feet[0] - 5, feet[1], feet[2] + 2];
    let at = |k: i32| [origin[0] + k, origin[1], origin[2]];
    p.command(&format!("structure load loot:row {}", place(origin))).await?;
    if brush_only {
        p.command("gamemode survival").await?;
        p.command("give @s brush").await?;
        let selected = p.bot.select_hotbar(0);
        p.act("select the brush", selected).await?;
        p.brush("suspicious sand", at(8), 200).await?;
        p.brush("suspicious gravel, a while", at(10), 30).await?;
        p.settle(Duration::from_secs(6)).await?;
        p.brush("suspicious gravel again", at(10), 200).await?;
        p.settle(Duration::from_secs(2)).await?;
        p.bot.disconnect().await;
        return Ok(());
    }

    // 1. Chest 0 opened: the slots its loot fills.
    p.open("open chest 0", at(0)).await?;
    // 2. Chest 2 with a hopper under it; then the hopper and the chest opened.
    let under = [at(2)[0], origin[1] - 1, origin[2]];
    p.command(&format!("setblock {} hopper [\"facing_direction\"=0]", place(under))).await?;
    p.settle(Duration::from_secs(4)).await?;
    p.open("open the hopper under chest 2", under).await?;
    p.open("open chest 2", at(2)).await?;
    // 3. Chest 4 with comparators on both sides along z; then the chest opened.
    let (south, north) = ([at(4)[0], origin[1], origin[2] + 1], [at(4)[0], origin[1], origin[2] - 1]);
    p.command(&format!("setblock {} unpowered_comparator [\"minecraft:cardinal_direction\"=\"south\"]", place(south))).await?;
    p.command(&format!("setblock {} unpowered_comparator [\"minecraft:cardinal_direction\"=\"north\"]", place(north))).await?;
    p.settle(Duration::from_secs(3)).await?;
    p.open("open chest 4", at(4)).await?;

    // 4. The decorated pot broken in survival, by hand.
    p.command("gamemode survival").await?;
    let broken = p.bot.break_block(at(6)).await;
    p.act("break the pot (survival)", broken).await?;
    p.settle(Duration::from_secs(2)).await?;
    // 4b. A second row's pot broken in creative.
    p.command("gamemode creative").await?;
    let second = [origin[0], origin[1], origin[2] + 3];
    p.command(&format!("structure load loot:row {}", place(second))).await?;
    let broken = p.bot.break_block([second[0] + 6, second[1], second[2]]).await;
    p.act("break the pot (creative)", broken).await?;
    p.settle(Duration::from_secs(2)).await?;
    p.command("gamemode survival").await?;
    p.bot.disconnect().await;
    Ok(())
}
