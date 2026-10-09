//! A server for drill probes, as plain as the test BDS they are compared with: offline, flat
//! ground four blocks deep under the drills (a flat world's, so that what a drill digs has
//! something under it), everyone an operator in creative mode, vanilla's commands, nothing
//! protected. `tools/sapling-drill.sh` runs a probe against it.
//! `cargo run -p sapling --example drill_server -- [port]` (default 19190)
use sapling::{GameMode, Handler, Permission, Player, Server, ShardCtx, World};

/// The ground's corners: wider than the probes' drills reach.
const GROUND: ([i32; 3], [i32; 3]) = ([-96, 60, -48], [96, 63, 48]);

struct Operators;

impl Handler for Operators {
    fn on_join(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        ctx.set_permission(player.id, Permission::Operator);
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into())).init();
    let port = std::env::args().nth(1).and_then(|port| port.parse().ok()).unwrap_or(19190);
    let mut ground = World::void();
    let stone = ground.block_id("minecraft:stone").ok_or("no stone")?;
    ground.fill(GROUND.0, GROUND.1, stone);
    let builder = Server::builder().bind(([127, 0, 0, 1], port).into()).name("sapling drills").online_mode(false).game_mode(GameMode::Creative);
    let builder = builder.world(ground, [0.5, 64.0, 0.5]);
    let server = builder.handler(Operators).vanilla_commands().start().await?;
    tracing::info!(port, "started");
    tokio::signal::ctrl_c().await?;
    server.shutdown().await;
    Ok(())
}
