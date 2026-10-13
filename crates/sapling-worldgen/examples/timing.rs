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
    let [seed, x0, z0, x1, z1] = args[..] else { panic!("usage: timing <seed> x0 z0 x1 z1") };
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
