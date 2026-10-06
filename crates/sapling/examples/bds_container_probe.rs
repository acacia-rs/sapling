//! Shows what a vanilla server does with hoppers, shulker boxes, furnace slots and experience orbs:
//! how often a hopper moves an item into a chest the prober looks into, what a broken shulker box
//! drops (filled and empty, survival and creative), whether a furnace takes an item placed into its
//! result or non-fuel into its fuel, and which packets carry experience orbs from a furnace to the
//! player. The ground truth for sapling's containers (docs/research/hoppers.md, shulker-ender.md,
//! furnaces.md, experience.md, "Not settled").
//! `cargo run -p sapling --example bds_container_probe -- <server> > probe.txt` (offline, operators)
use std::time::{Duration, Instant};

use acacia_bot::client::{Client, PacketFilter};
use acacia_bot::proto::{Packet, RawPacket, packets};
use acacia_bot::{Bot, BotConfig, BotEvent};

/// Characters of a decoded packet shown; item tags are long.
const SHOWN: usize = 1500;
const SETTLE: Duration = Duration::from_millis(1500);

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
    InventorySlot, InventoryContent, ItemStackResponse, ContainerOpen, ContainerClose, BlockEvent, LevelSoundEvent, AddItemEntity, TakeItemEntity, AddEntity,
    RemoveEntity, MoveEntity, MoveEntityDelta, SetEntityMotion, SetEntityData, UpdateAttributes, LevelEvent, Disconnect,
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

    /// Runs the bot's own action, then prints what follows for a while.
    async fn act<T: std::fmt::Debug, E: std::fmt::Debug>(&mut self, label: &str, result: Result<T, E>) -> Result<(), String> {
        println!("## {label} -> {result:?}");
        self.settle(SETTLE).await
    }
}

fn place(pos: [i32; 3]) -> String {
    format!("{} {} {}", pos[0], pos[1], pos[2])
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = std::env::args().nth(1).ok_or("usage: bds_container_probe <server>")?;
    let config = BotConfig { physics: true, subscribe: PacketFilter::all(), ..BotConfig::default() };
    let mut bot = Bot::connect(Client::builder(&server).offline("Prober"), config).await?;
    bot.wait_ticks(60).await?;
    for line in ["gamemode creative", "clear @s", "xp -100L @s", "fill ~-6 ~-1 ~-6 ~6 ~-1 ~6 stone", "fill ~-6 ~ ~-6 ~6 ~4 ~6 air"] {
        bot.client().command(line);
        bot.wait_ticks(20).await?;
    }
    while let Ok(Some(_)) = tokio::time::timeout(Duration::from_millis(5), bot.next()).await {}
    let feet = bot.movement().and_then(|m| m.position()).ok_or("the bot has no position")?.map(|c| c.floor() as i32);
    let at = |dx: i32, dy: i32, dz: i32| [feet[0] + dx, feet[1] + dy, feet[2] + dz];
    println!("## feet {feet:?}");
    let mut p = Probe { start: Instant::now(), bot };

    // 1. A hopper under a chest of ten diamonds, facing west into a chest the prober looks into:
    // the times of the InventorySlot changes are its rate.
    let (above, hopper, target) = (at(3, 1, 0), at(3, 0, 0), at(2, 0, 0));
    p.command(&format!("setblock {} chest", place(target))).await?;
    p.command(&format!("setblock {} chest", place(above))).await?;
    p.command(&format!("replaceitem block {} slot.container 0 diamond 10", place(above))).await?;
    let opened = p.bot.open_container_at(target).await.map(|c| c.slots.len());
    p.act("open the target chest", opened).await?;
    println!("## hopper placed now");
    p.command(&format!("setblock {} hopper [\"facing_direction\"=4]", place(hopper))).await?;
    p.settle(Duration::from_secs(8)).await?;
    let closed = p.bot.close_container().await;
    p.act("close", closed).await?;

    // 2. Furnace slots: stone into the result, stone into the fuel.
    p.command("gamemode survival").await?;
    p.command("give @s stone 4").await?;
    let furnace = at(-3, 0, 0);
    p.command(&format!("setblock {} furnace", place(furnace))).await?;
    let opened = p.bot.open_container_at(furnace).await.map(|c| c.slots.len());
    p.act("open the furnace", opened).await?;
    let into_result = p.bot.put_in_container(0, 2).await;
    p.act("stone into the result (slot 2)", into_result).await?;
    p.command("give @s stone 4").await?;
    let into_fuel = p.bot.put_in_container(0, 1).await;
    p.act("stone into the fuel (slot 1)", into_fuel).await?;
    let closed = p.bot.close_container().await;
    p.act("close", closed).await?;

    // 3. Experience: three iron ingots taken from a furnace's result; the orbs as they move.
    p.command(&format!("replaceitem block {} slot.container 2 iron_ingot 3", place(furnace))).await?;
    let opened = p.bot.open_container_at(furnace).await.map(|c| c.slots.len());
    p.act("open the furnace again", opened).await?;
    let taken = p.bot.move_to_inventory(2).await;
    p.act("take the ingots", taken).await?;
    let closed = p.bot.close_container().await;
    p.act("close", closed).await?;
    p.settle(Duration::from_secs(8)).await?;
    p.command(&format!("summon xp_orb {}", place(at(0, 1, 4)))).await?;
    p.settle(Duration::from_secs(8)).await?;

    // 4. Shulker boxes broken: filled and empty, in survival and in creative.
    let boxes = [at(0, 0, 2), at(1, 0, 2), at(0, 0, -2), at(1, 0, -2)];
    for pos in boxes {
        p.command(&format!("setblock {} red_shulker_box", place(pos))).await?;
    }
    for pos in [boxes[0], boxes[2]] {
        p.command(&format!("replaceitem block {} slot.container 4 diamond 5", place(pos))).await?;
    }
    for (label, pos) in [("survival, filled", boxes[0]), ("survival, empty", boxes[1])] {
        let broken = p.bot.break_block(pos).await;
        p.act(label, broken).await?;
    }
    p.command("gamemode creative").await?;
    for (label, pos) in [("creative, filled", boxes[2]), ("creative, empty", boxes[3])] {
        let broken = p.bot.break_block(pos).await;
        p.act(label, broken).await?;
    }
    p.bot.disconnect().await;
    Ok(())
}
