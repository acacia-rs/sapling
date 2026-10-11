//! A chunk's blocks after each stage of ours, generated on every core and handed over in order.
//! `SAPLING_ORACLE_THREADS` sets the worker count (default: every core).

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, mpsc};

use sapling_worldgen::structure::{BoundingBox, StructureKind};
use sapling_worldgen::{ChunkBiomes, ChunkShape, OverworldGenerator};

/// Chunks a worker may finish before the one being compared; each holds five copies of its blocks.
const AHEAD: usize = 64;

pub struct Stages {
    pub surfaced: ChunkShape,
    /// Carved.
    pub loaded: ChunkShape,
    pub structured: ChunkShape,
    /// Decorated: lakes and the like.
    pub laked: ChunkShape,
    /// With features: the finished chunk.
    pub ours: ChunkShape,
    pub areas: Vec<(StructureKind, (i32, i32), BoundingBox)>,
    pub biomes: Arc<ChunkBiomes>,
}

impl Stages {
    fn new(generator: &OverworldGenerator, (cx, cz): (i32, i32)) -> Self {
        let surfaced = generator.surfaced_blocks(cx, cz);
        let mut loaded = surfaced.clone();
        generator.carve(&mut loaded, cx, cz);
        let mut structured = loaded.clone();
        generator.place_structures(&mut structured, cx, cz);
        let mut laked = structured.clone();
        generator.decorate(&mut laked, cx, cz);
        // Each source places its own lake first (feature/lake.rs); `laked` is the lake alone, for the judge.
        let mut ours = structured.clone();
        generator.place_features(&mut ours, cx, cz);
        Self { surfaced, loaded, structured, laked, ours, areas: generator.structure_areas(cx, cz), biomes: generator.chunk_biomes(cx, cz) }
    }

    /// Calls `each` with every chunk's stages, in the order of `chunks`.
    pub fn in_order(generator: &OverworldGenerator, chunks: &[(i32, i32)], mut each: impl FnMut((i32, i32), Stages)) {
        let threads = std::env::var("SAPLING_ORACLE_THREADS").ok().and_then(|n| n.parse().ok()).unwrap_or_else(|| std::thread::available_parallelism().map_or(1, |n| n.get()));
        let next = AtomicUsize::new(0);
        let (compared, moved) = (Mutex::new(0usize), Condvar::new());
        let release = |upto: usize| {
            *compared.lock().unwrap() = upto;
            moved.notify_all();
        };
        let (send, receive) = mpsc::channel();
        std::thread::scope(|scope| {
            for _ in 0..threads.max(1) {
                let (send, next, compared, moved) = (send.clone(), &next, &compared, &moved);
                scope.spawn(move || {
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        if i >= chunks.len() {
                            break;
                        }
                        drop(moved.wait_while(compared.lock().unwrap(), |compared| i >= compared.saturating_add(AHEAD)).unwrap());
                        // A panic is sent on: the workers waiting above would otherwise never be released.
                        if send.send((i, catch_unwind(AssertUnwindSafe(|| Stages::new(generator, chunks[i]))))).is_err() {
                            break;
                        }
                    }
                });
            }
            drop(send);
            let mut early = BTreeMap::new();
            for (at, &chunk) in chunks.iter().enumerate() {
                while !early.contains_key(&at) {
                    let (i, stages) = receive.recv().expect("workers end after the last chunk");
                    early.insert(i, stages);
                }
                match early.remove(&at).unwrap() {
                    Ok(stages) => each(chunk, stages),
                    Err(panic) => {
                        release(usize::MAX);
                        resume_unwind(panic);
                    }
                }
                release(at + 1);
            }
        });
    }
}
