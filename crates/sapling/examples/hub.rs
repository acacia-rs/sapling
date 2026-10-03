//! A hub that logs who comes and goes, in a vanilla world folder or, without one, on a stone
//! platform in the void.
//! `cargo run -p sapling --example hub -- [port] [world folder] [--offline] [--no-cache]` (default port 19132,
//! online mode). `RUST_LOG=debug` shows refused logins and ended connections.
use std::path::Path;

use sapling::{GameMode, Handler, Player, Server, ShardCtx, World};

struct Greeter;

impl Handler for Greeter {
    fn on_join(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        tracing::info!(name = %player.name, xuid = %player.xuid, uuid = %player.uuid, online = ctx.player_count(), "joined");
    }

    fn on_quit(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        tracing::info!(name = %player.name, online = ctx.player_count(), "left");
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    tracing_subscriber::fmt().with_env_filter(filter).init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let offline = args.iter().any(|a| a == "--offline");
    let positional = || args.iter().filter(|a| !a.starts_with("--"));
    let port = positional().find_map(|a| a.parse::<u16>().ok()).unwrap_or(19132);
    let folder = positional().find(|a| a.parse::<u16>().is_err());

    let mut builder = Server::builder().bind(([0, 0, 0, 0], port).into()).name("sapling hub").game_mode(GameMode::Adventure).online_mode(!offline).blob_cache(!args.iter().any(|a| a == "--no-cache"));
    if let Some(folder) = folder {
        let opened = World::open_folder(Path::new(folder))?;
        tracing::info!(folder, name = ?opened.name, spawn = ?opened.spawn, "opened the world");
        builder = builder.world(opened.world, opened.spawn);
    }
    let server = builder.handler(Greeter).start().await?;
    tokio::signal::ctrl_c().await?;
    tracing::info!("shutting down");
    server.shutdown().await;
    Ok(())
}
