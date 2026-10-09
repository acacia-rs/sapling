//! Joins a vanilla server with a bot and compares the terrain it is sent with a world folder as
//! sapling-leveldb reads it. Run against a server started on a copy of a folder sapling wrote
//! (`rewrite` in sapling-leveldb), it shows whether the game reads what sapling writes; the
//! folder given here must be one the server has not opened, since the game rewrites its own.
//! `cargo run -p sapling --example bds_world_check -- <server> <world folder>` (an offline-mode
//! server); exits with an error unless every saved column arrived and matches.
use std::path::PathBuf;
use std::time::{Duration, Instant};

use acacia_bot::client::Client;
use acacia_bot::{Bot, BotConfig};
use acacia_world::{BlockAccess, BlockRegistry, Chunk};
use sapling_leveldb::LevelDbWorld;

/// How long the server gets to start listening, and the saved columns to arrive.
const PATIENCE: Duration = Duration::from_secs(90);
/// Chunks around the spawn the bot asks for: the saved columns must lie within it.
const VIEW_RADIUS: i32 = 8;
/// Differences printed in full.
const SHOWN: usize = 12;

async fn connect(server: &str) -> Result<Bot, Box<dyn std::error::Error>> {
    let deadline = Instant::now() + PATIENCE;
    // The game sends more than the columns next to the spawn only to a player that sends input.
    let config = || BotConfig {
        physics: true,
        ..BotConfig::default()
    };
    loop {
        match Bot::connect(
            Client::builder(server)
                .offline("WorldCheck")
                .chunk_radius(VIEW_RADIUS),
            config(),
        )
        .await
        {
            Ok(bot) => return Ok(bot),
            Err(error) if Instant::now() > deadline => return Err(error.into()),
            Err(_) => tokio::time::sleep(Duration::from_secs(2)).await,
        }
    }
}

fn arrived(bot: &Bot, saved: &[Chunk]) -> usize {
    let Some(view) = bot.world().and_then(|world| world.view()) else {
        return 0;
    };
    saved
        .iter()
        .filter(|chunk| view.chunk(chunk.x, chunk.z).is_some())
        .count()
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [server, folder] = &args[..] else {
        return Err("usage: bds_world_check <server> <world folder>".into());
    };
    let world = LevelDbWorld::open_in_memory(&PathBuf::from(folder))?;
    let mut saved = Vec::new();
    for key in world
        .columns()?
        .into_iter()
        .filter(|key| key.dimension == 0)
    {
        saved.push(world.load_column(key)?.ok_or("a listed column")?.chunk);
    }

    let mut bot = connect(server).await?;
    let deadline = Instant::now() + PATIENCE;
    while arrived(&bot, &saved) < saved.len() && Instant::now() < deadline {
        bot.wait_ticks(10).await?;
    }
    // Sections follow their column's announcement.
    bot.wait_ticks(60).await?;

    let registry = BlockRegistry::vanilla();
    let describe = |id: u32| {
        registry.get(id).map_or(format!("#{id}"), |state| {
            format!("{}[{}]", state.name, state.properties)
        })
    };
    let view = bot
        .world()
        .and_then(|world| world.view())
        .ok_or("the bot has no chunk view")?;
    let (mut missing, mut differing, mut shown) = (0, 0, 0);
    for chunk in &saved {
        if view.chunk(chunk.x, chunk.z).is_none() {
            missing += 1;
            println!("column ({}, {}): not sent", chunk.x, chunk.z);
            continue;
        }
        let mut here = 0;
        for (x, z) in
            (0..16).flat_map(|x| (0..16).map(move |z| (chunk.x * 16 + x, chunk.z * 16 + z)))
        {
            for y in -64..320 {
                let pairs = [
                    ("block", chunk.block(x, y, z), view.block(x, y, z)),
                    ("liquid", chunk.liquid(x, y, z), view.liquid(x, y, z)),
                ];
                for (layer, wrote, sent) in
                    pairs.into_iter().filter(|(_, wrote, sent)| wrote != sent)
                {
                    here += 1;
                    if shown < SHOWN {
                        shown += 1;
                        println!(
                            "  ({x}, {y}, {z}) {layer}: saved {} but sent {}",
                            describe(wrote),
                            describe(sent)
                        );
                    }
                }
            }
        }
        differing += here;
        println!("column ({}, {}): {here} differ", chunk.x, chunk.z);
    }
    bot.disconnect().await;
    println!(
        "{} columns saved, {missing} not sent, {differing} blocks differ",
        saved.len()
    );
    if missing + differing > 0 {
        return Err("the server's world is not the saved one".into());
    }
    Ok(())
}
