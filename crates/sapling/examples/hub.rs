//! A hub: a stone platform in the void that logs who comes and goes.
//! `cargo run -p sapling --example hub -- [port] [--offline]` (default port 19132, online mode).
//! `RUST_LOG=debug` shows refused logins and ended connections.
use sapling::{GameMode, Handler, Player, Server, ShardCtx};

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
    let port: u16 = args.iter().find(|a| !a.starts_with("--")).map_or(Ok(19132), |p| p.parse())?;

    let server = Server::builder()
        .bind(([0, 0, 0, 0], port).into())
        .name("sapling hub")
        .game_mode(GameMode::Adventure)
        .online_mode(!offline)
        .handler(Greeter)
        .start()
        .await?;
    tokio::signal::ctrl_c().await?;
    tracing::info!("shutting down");
    server.shutdown().await;
    Ok(())
}
