//! A hub that announces who comes and goes and logs the chat, in a vanilla world folder or,
//! without one, on a stone platform in the void.
//! `cargo run -p sapling --example hub -- [port] [world folder] [--offline] [--no-cache]` (default port 19132,
//! online mode). `RUST_LOG=debug` shows refused logins and ended connections.
use std::path::Path;

use sapling::{ChatOutcome, GameMode, Handler, Player, Server, ShardCtx, World};

struct Greeter;

impl Handler for Greeter {
    fn on_join(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        tracing::info!(name = %player.name, xuid = %player.xuid, uuid = %player.uuid, online = ctx.player_count(), "joined");
        ctx.broadcast(&format!("§e{} joined the game", player.name));
        let others: Vec<String> = ctx.players().filter(|p| p.id != player.id).map(|p| p.name).collect();
        if !others.is_empty() {
            ctx.send_message(player.id, &format!("§7Here: {}", others.join(", ")));
        }
    }

    fn on_quit(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        tracing::info!(name = %player.name, online = ctx.player_count(), "left");
        ctx.broadcast(&format!("§e{} left the game", player.name));
    }

    fn on_chat(&mut self, ctx: &mut ShardCtx<'_>, player: &Player, message: &str) -> ChatOutcome {
        let at = ctx.position(player.id).map(|p| p.feet.map(|c| c.floor() as i32));
        tracing::info!(name = %player.name, ?at, message, "chat");
        ChatOutcome::Allow
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
