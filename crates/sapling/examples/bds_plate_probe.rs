//! Pressure plates as a vanilla server works them, the drill `bds_mechanism_probe` could not get to
//! press: the actor walks (movement input, not a teleport) onto a plate `/setblock` wrote and onto
//! one it placed by hand with a lamp beside it, then a chicken is left on that one; after each step
//! `/testforblock` asks the server what the plate is. Ground truth for sapling's `blocks/plates.rs`.
//! `cargo run -p sapling --example bds_plate_probe -- <server>` (offline mode, operators by default)
#[path = "shared/dump.rs"]
mod dump;
#[path = "shared/probe.rs"]
#[allow(dead_code)]
mod probe;

use std::time::Duration;

use acacia_bot::interact::Face;
use acacia_bot::pathfind::Goal;
use probe::{Probe, connect};

const SHOWN: &[&str] = &["UpdateBlock", "UpdateBlockSynced", "LevelSoundEvent", "LevelEvent", "CommandOutput", "AddEntity", "RemoveEntity"];
const LONG: Duration = Duration::from_millis(3000);

fn at(origin: [i32; 3], [dx, dy, dz]: [i32; 3]) -> [i32; 3] {
    [origin[0] + dx, origin[1] + dy, origin[2] + dz]
}

fn coords([x, y, z]: [i32; 3]) -> String {
    format!("{x} {y} {z}")
}

impl Probe {
    /// Walks the actor to `to` with movement input.
    async fn walk(&mut self, label: &str, to: [i32; 3]) -> Result<(), String> {
        println!("## {label}: walk to {to:?}");
        let result = self.during(|bot| Box::pin(bot.goto(Goal::Block(to)))).await?;
        println!("## -> {result:?}");
        self.settle(LONG).await
    }

    /// Asks the server whether the plate at `pos` stores `signal`.
    async fn test_plate(&mut self, pos: [i32; 3], name: &str, signal: u8) -> Result<(), String> {
        self.command(&format!("testforblock {} {name} [\"redstone_signal\"={signal}]", coords(pos)), Duration::from_millis(600)).await
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = std::env::args().nth(1).ok_or("usage: bds_plate_probe <server>")?;
    let (mut actor, mut watcher) = (connect(&server, "PlateActor").await?, connect(&server, "PlateWatcher").await?);
    watcher.wait_ticks(60).await?;
    actor.wait_ticks(60).await?;
    let o = actor.movement().and_then(|m| m.position()).ok_or("the actor has no position")?.map(|c| c.floor() as i32);
    let mut p = Probe { start: std::time::Instant::now(), actor, watcher, shown: SHOWN };
    let (written, placed, lamp) = (at(o, [0, 0, 3]), at(o, [3, 0, 3]), at(o, [3, 0, 4]));
    p.setup(&[
        "gamemode creative".into(),
        "gamerule domobspawning false".into(),
        format!("fill {} {} dirt", coords(at(o, [-4, -1, -2])), coords(at(o, [6, -1, 6]))),
        format!("fill {} {} air", coords(at(o, [-4, 0, -2])), coords(at(o, [6, 3, 6]))),
        format!("setblock {} stone_pressure_plate", coords(written)),
        format!("setblock {} redstone_lamp", coords(lamp)),
        "clear @s".into(),
        "give @s wooden_pressure_plate".into(),
    ])
    .await?;
    p.actor.select_hotbar(0).map_err(|e| e.to_string())?;
    println!("## actor feet {o:?}");

    p.test_plate(written, "stone_pressure_plate", 0).await?;
    p.walk("onto the written stone plate", written).await?;
    p.test_plate(written, "stone_pressure_plate", 1).await?;
    p.walk("off it", o).await?;
    p.test_plate(written, "stone_pressure_plate", 0).await?;

    p.place_block("a wooden plate by hand", at(placed, [0, -1, 0]), Face::Up, LONG).await?;
    p.walk("onto the placed wooden plate", placed).await?;
    p.test_plate(placed, "wooden_pressure_plate", 1).await?;
    p.walk("off it", o).await?;
    p.test_plate(placed, "wooden_pressure_plate", 0).await?;

    p.command(&format!("summon chicken {}.5 {} {}.5", placed[0], placed[1], placed[2]), LONG).await?;
    p.test_plate(placed, "wooden_pressure_plate", 1).await?;
    p.command("kill @e[type=chicken]", LONG).await?;
    p.test_plate(placed, "wooden_pressure_plate", 0).await?;
    p.actor.disconnect().await;
    p.watcher.disconnect().await;
    Ok(())
}
