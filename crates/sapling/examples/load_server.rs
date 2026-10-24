//! A busy world for profiling many clients at once (acacia's tools/swarm-load.sh): offline, a stone
//! floor wider than a view, observer clocks each driving a sticky piston and a lamp, and strolling
//! pigs. Every player sees every other, so a crowd is load too.
//! `cargo run --release -p sapling --example load_server -- [port] [clocks] [mobs]` (19192, 0, 0)
use sapling::{Block, BlockPos, Handler, Server, ShardCtx, World};

const FLOOR: ([i32; 3], [i32; 3]) = ([-160, 60, -160], [160, 63, 160]);
const SPAWN: [f32; 3] = [0.5, 64.0, 0.5];
/// Blocks between clocks, both ways, on a square centred on the spawn.
const SPACING: i32 = 8;
const MOB_SPREAD: f32 = 48.0;
/// Ticks after building to count lit lamps: about half are lit while the clocks run.
const CHECK_AFTER: u32 = 200;

struct Load {
    clocks: Vec<BlockPos>,
    mobs: usize,
    built: bool,
    ticks: u32,
}

impl Load {
    fn new(clocks: usize, mobs: usize) -> Self {
        let side = (clocks as f64).sqrt().ceil() as i32;
        let origin = -(side - 1) * SPACING / 2;
        let clocks = (0..clocks as i32).map(|i| [origin + i % side * SPACING, 64, origin + i / side * SPACING]).collect();
        Self { clocks, mobs, built: false, ticks: 0 }
    }

    /// Two observers watching each other pulse forever once one sees the other placed. The west
    /// one's output pushes a stone up with a sticky piston, the east one's lights a lamp.
    fn build_clock(ctx: &mut ShardCtx<'_>, [x, y, z]: BlockPos) -> usize {
        let observer = |facing: &str| Block::new("observer").with(format!("minecraft:facing_direction={facing}"));
        let parts = [
            ([x - 1, y, z], Block::new("sticky_piston").with("facing_direction=1")),
            ([x - 1, y + 1, z], Block::new("stone")),
            ([x + 2, y, z], Block::new("redstone_lamp")),
            ([x, y, z], observer("east")),
            ([x + 1, y, z], observer("west")),
        ];
        parts.iter().filter(|(pos, block)| ctx.update_block(*pos, block)).count()
    }

    fn spawn_mobs(&self, ctx: &mut ShardCtx<'_>) -> usize {
        let side = (self.mobs as f32).sqrt().ceil().max(1.0);
        let step = 2.0 * MOB_SPREAD / side;
        (0..self.mobs)
            .filter(|i| {
                let (col, row) = ((*i as f32) % side, (*i as f32 / side).floor());
                ctx.spawn_entity("minecraft:pig", [-MOB_SPREAD + col * step + 0.5, 64.0, -MOB_SPREAD + row * step + 0.5]).is_some()
            })
            .count()
    }
}

impl Handler for Load {
    // Built once, on the first tick every clock's cell is loaded: a running clock's piston
    // cells are moving blocks, so placing again would never all succeed.
    fn on_tick(&mut self, ctx: &mut ShardCtx<'_>) {
        if self.built {
            self.ticks += 1;
            if self.ticks == CHECK_AFTER {
                let lit = self.clocks.iter().filter(|[x, y, z]| ctx.block([x + 2, *y, *z]).is_some_and(|b| b.name.ends_with("lit_redstone_lamp"))).count();
                tracing::info!(lit, of = self.clocks.len(), "lamps lit");
            }
            return;
        }
        if self.clocks.iter().any(|pos| ctx.block(*pos).is_none()) {
            return;
        }
        self.built = true;
        let blocks: usize = self.clocks.iter().map(|pos| Self::build_clock(ctx, *pos)).sum();
        let mobs = self.spawn_mobs(ctx);
        tracing::info!(clocks = self.clocks.len(), blocks, of = self.clocks.len() * 5, mobs, "load world built");
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into())).init();
    let arg = |n: usize, default: usize| std::env::args().nth(n).map_or(Ok(default), |a| a.parse());
    let (port, clocks, mobs) = (arg(1, 19192)? as u16, arg(2, 0)?, arg(3, 0)?);
    let mut world = World::void();
    let stone = world.block_id("minecraft:stone").ok_or("no stone")?;
    world.fill(FLOOR.0, FLOOR.1, stone);
    let server = Server::builder()
        .bind(([127, 0, 0, 1], port).into())
        .name("sapling load")
        .online_mode(false)
        .max_players(400)
        .world(world, SPAWN)
        .handler(Load::new(clocks, mobs))
        .start()
        .await?;
    tracing::info!(port, clocks, mobs, "started");
    tokio::signal::ctrl_c().await?;
    server.shutdown().await;
    Ok(())
}
