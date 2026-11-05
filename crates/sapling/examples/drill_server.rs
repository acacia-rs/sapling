//! A server for drill probes, as plain as the test BDS they are compared with: offline, flat
//! ground four blocks deep under the drills (a flat world's, so that what a drill digs has
//! something under it), everyone an operator in creative mode, vanilla's commands, nothing
//! protected. `tools/sapling-drill.sh` runs a probe against it. `SAPLING_DRILL_WORLD` picks
//! another overworld: `strongholds`, `generated`, `cold` or `lowflat` (also `open`, below).
//! `cargo run -p sapling --example drill_server -- [port]` (default 19190)
#[path = "shared/clock.rs"]
#[allow(dead_code)]
mod clock;

use sapling::{ChunkPos, END, GameMode, Generated, Handler, MemoryStorage, NETHER, Permission, Player, Server, ShardCtx, World, WorldStorage};

/// The ground's corners: wider than the probes' drills reach.
const GROUND: ([i32; 3], [i32; 3]) = ([-96, 60, -48], [96, 63, 48]);
const FLAT: ([i32; 3], [i32; 3]) = ([-96, -64, -48], [224, -61, 48]);
const NETHER_SEED: i64 = 20261018;
/// `SAPLING_DRILL_WORLD=lowflat` (or `open`): the flat ground alone, its surface and sky the test BDS's, for raid
/// waves (village_drill.rs) and rain (docs/research/mob-damage.md); GROUND's roof shades the drills below it.
const OPEN_SPAWN: [f32; 3] = [0.5, (FLAT.1[1] + 1) as f32, 0.5];

/// `SAPLING_DRILL_WORLD=cold`: `open`'s ground in ice plains, as a BDS flat world whose `FlatWorldLayers`
/// name biome 12 (drill `u1cold`); columns past the ground are void ice plains.
fn cold_ground() -> std::sync::Arc<MemoryStorage> {
    const ICE_PLAINS: u32 = 12;
    let registry = sapling_world::BlockRegistry::vanilla();
    let stone = registry.states_of("minecraft:stone").next().expect("stone").0;
    let storage = std::sync::Arc::new(MemoryStorage::default());
    for cx in (FLAT.0[0] >> 4)..=(FLAT.1[0] >> 4) {
        for cz in (FLAT.0[2] >> 4)..=(FLAT.1[2] >> 4) {
            let mut chunk = sapling_world::Chunk::empty(cx, cz, sapling_world::Dimension::overworld(registry.air_id()));
            chunk.fill_biomes(ICE_PLAINS);
            for (x, z, y) in (0..16).flat_map(|x| (0..16).flat_map(move |z| (FLAT.0[1]..=FLAT.1[1]).map(move |y| (x, z, y)))) {
                chunk.set(x, y, z, 0, stone);
            }
            storage.store_column(ChunkPos::new(cx, cz), 0, &sapling_world::StoredColumn::of(chunk)).expect("stored");
        }
    }
    storage
}

/// `SAPLING_DRILL_WORLD=generated`: the seed's terrain, for timing joins over a slow link
/// (acacia's tools/netem-pacing.sh); release builds only, as below.
const GENERATED_SPAWN: [f32; 3] = [0.5, 120.0, 0.5];

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
    let open = matches!(std::env::var("SAPLING_DRILL_WORLD").as_deref(), Ok("lowflat" | "open"));
    if !open {
        ground.fill(GROUND.0, GROUND.1, stone);
    }
    // Where a BDS flat world's ground is, for drills that name places (portal_drill.rs).
    ground.fill(FLAT.0, FLAT.1, stone);
    // Portals are made with the world seed's draws, as on the test BDS.
    ground.level_mut().seed = NETHER_SEED;
    let builder = Server::builder().bind(([127, 0, 0, 1], port).into()).name("sapling drills").online_mode(false).game_mode(GameMode::Creative);
    // The probes' bots and settles run `ACACIA_TICK_SCALE` times faster too (shared/clock.rs).
    let builder = builder.tick(clock::scaled(sapling_shard::TICK));
    // `SAPLING_TICK_DISTANCE`: the BDS copy's `tick-distance` the drill is compared with (default 4).
    let builder = builder.tick_distance(std::env::var("SAPLING_TICK_DISTANCE").ok().and_then(|chunks| chunks.parse().ok()).unwrap_or(4));
    // `SAPLING_DRILL_SEED`: with the generated world, another seed for every dimension, a BDS default world's
    // (twelfth3_drill.rs).
    let generated = std::env::var("SAPLING_DRILL_WORLD").as_deref() == Ok("generated");
    let seed = std::env::var("SAPLING_DRILL_SEED").ok().and_then(|seed| seed.parse().ok()).filter(|_| generated).unwrap_or(NETHER_SEED);
    let builder = match std::env::var("SAPLING_DRILL_WORLD").as_deref() {
        Ok("strongholds") => builder.world(World::with_storage(std::sync::Arc::new(Strongholds(Generated::with_areas(MemoryStorage::default(), NETHER_SEED, 1)))), [0.5, 64.0, 0.5]),
        Ok("cold") => {
            let mut cold = World::with_storage(cold_ground());
            cold.level_mut().seed = NETHER_SEED;
            builder.world(cold, OPEN_SPAWN)
        }
        Ok("generated") => builder.world(World::with_storage(std::sync::Arc::new(Generated::with_areas(MemoryStorage::default(), seed, 2))), GENERATED_SPAWN),
        // Joining over the void, the probes' first commands went unanswered.
        _ => builder.world(ground, if open { OPEN_SPAWN } else { [0.5, 64.0, 0.5] }),
    };
    // The test BDS's Nether, generated from its seed (tools/bds-drill.sh worlds: bds-nether's level-seed).
    let nether = World::with_storage_in(std::sync::Arc::new(Generated::nether(MemoryStorage::default(), seed, 2)), NETHER);
    let end = World::with_storage_in(std::sync::Arc::new(Generated::end(MemoryStorage::default(), seed, 2)), END);
    let builder = builder.nether(nether).end(end);
    let server = builder.handler(Operators).vanilla_commands().start().await?;
    tracing::info!(port, "started");
    tokio::signal::ctrl_c().await?;
    server.shutdown().await;
    Ok(())
}
