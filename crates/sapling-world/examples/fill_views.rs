//! `fill_views <players> [radius] [spread] [seed]`: a generated world served to `players` players
//! standing `spread` chunks apart in x, each asking for its view circle as a shard does
//! (sapling-shard's `View::take_ready`: nearest first, `BUDGET` columns a tick, twice that asked
//! about), in 50 ms ticks. Prints when each player could spawn and when its view was full, the wall
//! time per column, the heap allocations and a hash of every block (tools/bench.sh `views*`).

use std::alloc::{GlobalAlloc, Layout, System};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::time::{Duration, Instant};

use sapling_world::{ChunkPos, Generated, MemoryStorage, World};

struct Counting;

static ALLOCS: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Relaxed);
        BYTES.fetch_add(layout.size() as u64, Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Relaxed);
        BYTES.fetch_add(new_size as u64, Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

/// `ShardConfig::columns_per_tick`'s default.
const BUDGET: usize = 8;
/// sapling-shard's `SPAWN_RADIUS`.
const SPAWN_RADIUS: i32 = 3;
const TICK: Duration = Duration::from_millis(50);

struct Player {
    center: ChunkPos,
    /// Columns not sent yet, nearest first.
    queue: Vec<ChunkPos>,
    spawned: Option<Duration>,
    filled: Option<Duration>,
}

fn distance_sq(a: ChunkPos, b: ChunkPos) -> i32 {
    (a.x - b.x).pow(2) + (a.z - b.z).pow(2)
}

impl Player {
    fn new(center: ChunkPos, radius: i32) -> Self {
        let square = (-radius..=radius).flat_map(|dx| (-radius..=radius).map(move |dz| ChunkPos::new(center.x + dx, center.z + dz)));
        let mut queue: Vec<_> = square.filter(|p| distance_sq(*p, center) <= radius * radius).collect();
        queue.sort_by_key(|p| (distance_sq(*p, center), p.x, p.z));
        Self { center, queue, spawned: None, filled: None }
    }

    /// `View::take_ready` with `World::request` as the test.
    fn tick(&mut self, world: &mut World, now: Duration) {
        let (mut taken, mut asked) = (0, 0);
        self.queue.retain(|&pos| {
            if asked == BUDGET * 2 || taken == BUDGET {
                return true;
            }
            asked += 1;
            let ready = world.request(pos);
            taken += usize::from(ready);
            !ready
        });
        let waiting_near = self.queue.first().is_some_and(|nearest| distance_sq(*nearest, self.center) <= SPAWN_RADIUS * SPAWN_RADIUS);
        if !waiting_near && self.spawned.is_none() {
            self.spawned = Some(now);
        }
        if self.queue.is_empty() && self.filled.is_none() {
            self.filled = Some(now);
        }
    }
}

fn main() {
    let args: Vec<i64> = std::env::args().skip(1).map(|a| a.parse().expect("a number")).collect();
    let arg = |i: usize, default: i64| args.get(i).copied().unwrap_or(default);
    let (players, radius, spread, seed) = (arg(0, 1) as i32, arg(1, 8) as i32, arg(2, 64) as i32, arg(3, 20261003));
    let mut world = World::with_storage(Arc::new(Generated::new(MemoryStorage::default(), seed)));
    let mut players: Vec<Player> = (0..players).map(|i| Player::new(ChunkPos::new(i * spread, 0), radius)).collect();
    let columns: usize = players.iter().map(|p| p.queue.len()).sum();

    let start = Instant::now();
    let mut ticks = 0u32;
    while players.iter().any(|p| p.filled.is_none()) {
        world.poll();
        let now = start.elapsed();
        players.iter_mut().for_each(|p| p.tick(&mut world, now));
        ticks += 1;
        std::thread::sleep((TICK * ticks).saturating_sub(start.elapsed()));
    }
    let wall = start.elapsed();
    println!("fill cpu {:.2} s, peak rss {} MB", cpu_seconds(), peak_rss_mb());

    let secs = |times: Vec<Duration>| times.iter().map(|t| format!("{:.2}", t.as_secs_f64())).collect::<Vec<_>>().join(" ");
    println!("spawned s {}", secs(players.iter().filter_map(|p| p.spawned).collect()));
    println!("filled s {}", secs(players.iter().filter_map(|p| p.filled).collect()));
    println!("{columns} columns, {} players, radius {radius}, {:.2} s, {:.2} ms per column", players.len(), wall.as_secs_f64(), wall.as_secs_f64() * 1000.0 / columns as f64);
    println!("allocations {} MB {} hash {:016x}", ALLOCS.load(Relaxed), BYTES.load(Relaxed) >> 20, block_hash(&world, &players, radius));
}

/// User and system time of the process so far, from Linux's /proc (0 elsewhere).
fn cpu_seconds() -> f64 {
    let stat = std::fs::read_to_string("/proc/self/stat").unwrap_or_default();
    // Fields after the parenthesised command name; utime and stime are the 12th and 13th, in 1/100 s.
    let fields: Vec<&str> = stat.rsplit_once(')').map_or("", |(_, rest)| rest).split_whitespace().collect();
    let ticks = |i: usize| fields.get(i).and_then(|f| f.parse::<f64>().ok()).unwrap_or(0.0);
    (ticks(11) + ticks(12)) / 100.0
}

fn peak_rss_mb() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let hwm = status.lines().find_map(|l| l.strip_prefix("VmHWM:")).and_then(|v| v.trim().trim_end_matches("kB").trim().parse::<u64>().ok());
    hwm.unwrap_or(0) / 1024
}

/// Every block of every player's view, column by column in a fixed order.
fn block_hash(world: &World, players: &[Player], radius: i32) -> u64 {
    let mut positions: Vec<ChunkPos> = players.iter().flat_map(|p| Player::new(p.center, radius).queue).collect();
    positions.sort_by_key(|p| (p.x, p.z));
    let mut hasher = DefaultHasher::new();
    for pos in positions {
        for y in -64..320 {
            for z in pos.z * 16..pos.z * 16 + 16 {
                for x in pos.x * 16..pos.x * 16 + 16 {
                    (world.block(x, y, z), world.liquid(x, y, z)).hash(&mut hasher);
                }
            }
        }
    }
    hasher.finish()
}
