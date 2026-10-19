//! A world generated from a seed as BDS generates it, held in memory and lost on exit, in creative
//! mode. Players appear on the surface above (0, 0); columns are generated as they come into view.
//! Terrain, features and structures as BDS places them; the overworld only.
//! `cargo run --release -p sapling --example generated -- [port] [--seed=N] [--offline]`
//! (default port 19132, seed 20261003, online mode). Release: a debug build generates slowly.
use std::sync::Arc;

use sapling::{GameMode, Generated, MemoryStorage, Server, World};

const NAME: &str = "sapling generated";
const DEFAULT_SEED: i64 = 20261003;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    tracing_subscriber::fmt().with_env_filter(filter).init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let offline = args.iter().any(|a| a == "--offline");
    let seed = args.iter().find_map(|a| a.strip_prefix("--seed=")).map_or(Ok(DEFAULT_SEED), str::parse)?;
    let port = args.iter().filter(|a| !a.starts_with("--")).find_map(|a| a.parse::<u16>().ok()).unwrap_or(19132);

    let mut world = World::with_storage(Arc::new(Generated::new(MemoryStorage::default(), seed)));
    let spawn = world.load_spawn();
    tracing::info!(seed, ?spawn, "generating the world");
    let builder = Server::builder().bind(([0, 0, 0, 0], port).into()).name(NAME).online_mode(!offline).game_mode(GameMode::Creative);
    let server = builder.world(world, spawn).start().await?;
    tokio::signal::ctrl_c().await?;
    tracing::info!("shutting down");
    server.shutdown().await;
    Ok(())
}
