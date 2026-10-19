//! A server for drill probes, as plain as the test BDS they are compared with: offline, flat
//! ground four blocks deep under the drills (a flat world's, so that what a drill digs has
//! something under it), everyone an operator in creative mode, vanilla's commands, nothing
//! protected. `tools/sapling-drill.sh` runs a probe against it.
//! `cargo run -p sapling --example drill_server -- [port]` (default 19190)
use sapling::{ChunkPos, END, GameMode, Generated, Handler, MemoryStorage, NETHER, Permission, Player, Server, ShardCtx, World, WorldStorage};

/// The ground's corners: wider than the probes' drills reach.
const GROUND: ([i32; 3], [i32; 3]) = ([-96, 60, -48], [96, 63, 48]);
const FLAT: ([i32; 3], [i32; 3]) = ([-96, -64, -48], [224, -61, 48]);
const NETHER_SEED: i64 = 20261018;

/// `SAPLING_DRILL_WORLD=strongholds`: a void overworld whose eyes of ender find the seed's
/// strongholds, as on a BDS default world (bds-eye); a debug build generates terrain too slowly.
struct Strongholds(Generated<MemoryStorage>);

impl WorldStorage for Strongholds {
    fn load_column(&self, _: ChunkPos, _: i32) -> Result<Option<sapling_world::StoredColumn>, sapling_world::StorageError> {
        Ok(None)
    }
    fn store_column(&self, _: ChunkPos, _: i32, _: &sapling_world::StoredColumn) -> Result<(), sapling_world::StorageError> {
        Ok(())
    }
    fn locate_stronghold(&self, x: i32, z: i32) -> Option<[i32; 3]> {
        self.0.locate_stronghold(x, z)
    }
}

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
    // Where a BDS flat world's ground is, for drills that name places (portal_drill.rs).
    ground.fill(FLAT.0, FLAT.1, stone);
    // Portals are made with the world seed's draws, as on the test BDS.
    ground.level_mut().seed = NETHER_SEED;
    let builder = Server::builder().bind(([127, 0, 0, 1], port).into()).name("sapling drills").online_mode(false).game_mode(GameMode::Creative);
    let builder = match std::env::var("SAPLING_DRILL_WORLD").as_deref() {
        Ok("strongholds") => builder.world(World::with_storage(std::sync::Arc::new(Strongholds(Generated::with_areas(MemoryStorage::default(), NETHER_SEED, 1)))), [0.5, 64.0, 0.5]),
        _ => builder.world(ground, [0.5, 64.0, 0.5]),
    };
    // The test BDS's Nether, generated from its seed (tools/bds-drill.sh worlds: bds-nether's level-seed).
    let nether = World::with_storage_in(std::sync::Arc::new(Generated::nether(MemoryStorage::default(), NETHER_SEED, 2)), NETHER);
    let end = World::with_storage_in(std::sync::Arc::new(Generated::end(MemoryStorage::default(), NETHER_SEED, 2)), END);
    let builder = builder.nether(nether).end(end);
    let server = builder.handler(Operators).vanilla_commands().start().await?;
    tracing::info!(port, "started");
    tokio::signal::ctrl_c().await?;
    server.shutdown().await;
    Ok(())
}
