//! `timing <seed> <x0> <z0> <x1> <z1>`: milliseconds per column for `chunk_blocks` (every ported
//! stage) over a chunk range, one generator, cold caches; with the heap allocations made and a hash of
//! every block, which must not move under a pure speed change (tools/bench.sh).

use std::alloc::{GlobalAlloc, Layout, System};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::time::Instant;

use sapling_worldgen::OverworldGenerator;

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

fn main() {
    let args: Vec<i64> = std::env::args().skip(1).map(|a| a.parse().unwrap()).collect();
    match args[..] {
        [seed, x0, z0, x1, z1, threads] => return threaded(seed, [x0, z0, x1, z1].map(|v| v as i32), threads as usize, 1, 0),
        [seed, x0, z0, x1, z1, threads, areas, spread] => return threaded(seed, [x0, z0, x1, z1].map(|v| v as i32), threads as usize, areas as i32, spread as i32),
        _ => {}
    }
    let [seed, x0, z0, x1, z1] = args[..] else { panic!("usage: timing <seed> x0 z0 x1 z1 [threads [areas spread]]") };
    let generator = OverworldGenerator::new(seed);
    let start = Instant::now();
    let mut columns = 0;
    let mut hasher = DefaultHasher::new();
    for x in x0 as i32..=x1 as i32 {
        for z in z0 as i32..=z1 as i32 {
            let shape = generator.chunk_blocks(x, z);
            for y in sapling_worldgen::MIN_Y..sapling_worldgen::MIN_Y + sapling_worldgen::HEIGHT as i32 {
                for z in 0..16 {
                    for x in 0..16 {
                        shape.block(x, y, z).hash(&mut hasher);
                    }
                }
            }
            columns += 1;
        }
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / columns as f64;
    println!("{columns} columns, {ms:.2} ms per column");
    println!("allocations {} MB {} hash {:016x}",ALLOCS.load(Relaxed), BYTES.load(Relaxed) >> 20, hasher.finish());
}

/// The columns on `threads` threads sharing one generator; the hash sums each column's own hash, so
/// it does not depend on which thread finishes first. One area: each thread takes the next column of
/// the sweep. Several: the area repeated `spread` chunks apart in x, each thread sweeping whole areas.
fn threaded(seed: i64, [x0, z0, x1, z1]: [i32; 4], threads: usize, areas: i32, spread: i32) {
    let generator = OverworldGenerator::new(seed);
    let area = |offset: i32| -> Vec<(i32, i32)> { (x0 + offset..=x1 + offset).flat_map(|x| (z0..=z1).map(move |z| (x, z))).collect() };
    let units: Vec<Vec<(i32, i32)>> = if areas == 1 { area(0).into_iter().map(|c| vec![c]).collect() } else { (0..areas).map(|a| area(a * spread)).collect() };
    let columns: usize = units.iter().map(Vec::len).sum();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let sum = AtomicU64::new(0);
    let start = Instant::now();
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                while let Some(unit) = units.get(next.fetch_add(1, Relaxed)) {
                    for &(x, z) in unit {
                        sum.fetch_add(column_hash(&generator, x, z), Relaxed);
                    }
                }
            });
        }
    });
    let ms = start.elapsed().as_secs_f64() * 1000.0 / columns as f64;
    println!("{columns} columns on {threads} threads, {ms:.2} ms per column");
    println!("allocations {} MB {} hash {:016x}", ALLOCS.load(Relaxed), BYTES.load(Relaxed) >> 20, sum.load(Relaxed));
}

fn column_hash(generator: &OverworldGenerator, x: i32, z: i32) -> u64 {
    let shape = generator.chunk_blocks(x, z);
    let mut hasher = DefaultHasher::new();
    (x, z).hash(&mut hasher);
    for y in sapling_worldgen::MIN_Y..sapling_worldgen::MIN_Y + sapling_worldgen::HEIGHT as i32 {
        for z in 0..16 {
            for x in 0..16 {
                shape.block(x, y, z).hash(&mut hasher);
            }
        }
    }
    hasher.finish()
}
