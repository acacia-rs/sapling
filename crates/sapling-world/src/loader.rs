//! The I/O threads between a world and its storage: positions go in and loaded, encoded columns
//! come out; written columns go in and are saved. A shard only ever does a `try_recv`.
//! A storage that panics costs the job, not the thread: a world waits for every job it queued.

use std::collections::HashSet;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

use acacia_world::{BlockRegistry, Dimension};

use crate::ChunkPos;
use crate::column::Column;
use crate::storage::{StoredColumn, WorldStorage};

/// What every load and save needs besides the position.
#[derive(Clone)]
pub(crate) struct Source {
    pub storage: Arc<dyn WorldStorage>,
    pub registry: Arc<BlockRegistry>,
    pub dimension: i32,
    pub dim: Dimension,
}

impl Source {
    /// Reads and encodes one column on the calling thread. A column storage does not have is
    /// void; one it cannot read is [`Column::damaged`].
    pub fn load(&self, pos: ChunkPos) -> Column {
        let column = match catch_unwind(AssertUnwindSafe(|| self.storage.load_column(pos, self.dimension))) {
            Ok(Ok(Some(stored))) => Column::from_stored(stored),
            Ok(Ok(None)) => Column::empty(pos.x, pos.z, self.dim),
            Ok(Err(error)) => {
                tracing::error!(x = pos.x, z = pos.z, %error, "loading a column failed; serving it as void");
                Column::damaged(pos.x, pos.z, self.dim)
            }
            Err(_) => {
                tracing::error!(x = pos.x, z = pos.z, "storage panicked loading a column; serving it as void");
                Column::damaged(pos.x, pos.z, self.dim)
            }
        };
        column.warm(&self.registry);
        column
    }

    /// Writes one column on the calling thread; false, and logged, when storage refuses it.
    fn save(&self, pos: ChunkPos, column: &StoredColumn) -> bool {
        match catch_unwind(AssertUnwindSafe(|| self.storage.store_column(pos, self.dimension, column))) {
            Ok(Ok(())) => return true,
            Ok(Err(error)) => tracing::error!(x = pos.x, z = pos.z, %error, "saving a column failed"),
            Err(_) => tracing::error!(x = pos.x, z = pos.z, "storage panicked saving a column"),
        }
        false
    }
}

enum Job {
    Load(ChunkPos),
    Save(ChunkPos, StoredColumn),
}

/// One job the I/O threads are done with.
pub(crate) enum Finished {
    Loaded(ChunkPos, Column),
    Saved { pos: ChunkPos, ok: bool },
}

pub(crate) struct Loader {
    source: Source,
    jobs: Sender<Job>,
    /// Locked by its one reader only: the lock makes the world `Sync`, as an ECS component must be.
    finished: Mutex<Receiver<Finished>>,
    loading: HashSet<ChunkPos>,
    saving: HashSet<ChunkPos>,
}

impl Loader {
    /// Starts `threads` workers; they end when the loader is dropped and its queue is empty.
    pub fn spawn(source: Source, threads: usize) -> Self {
        let (jobs, queue) = channel::<Job>();
        let (deliver, finished) = channel();
        let queue = Arc::new(Mutex::new(queue));
        for index in 0..threads.max(1) {
            let (source, queue, deliver) = (source.clone(), queue.clone(), deliver.clone());
            let work = move || {
                loop {
                    // The lock covers only the wait for a job, not the job.
                    let next = queue.lock().expect("loader queue lock").recv();
                    let done = match next {
                        Ok(Job::Load(pos)) => Finished::Loaded(pos, source.load(pos)),
                        Ok(Job::Save(pos, chunk)) => Finished::Saved { pos, ok: source.save(pos, &chunk) },
                        Err(_) => return,
                    };
                    // Nobody listens once the world is gone, but the saves still queued must happen.
                    let _ = deliver.send(done);
                }
            };
            std::thread::Builder::new().name(format!("sapling-world-io-{index}")).spawn(work).expect("spawning a world I/O thread");
        }
        Self { source, jobs, finished: Mutex::new(finished), loading: HashSet::new(), saving: HashSet::new() }
    }

    pub fn source(&self) -> &Source {
        &self.source
    }

    /// Queues a load unless one is already on its way.
    pub fn request(&mut self, pos: ChunkPos) {
        if self.loading.insert(pos) {
            let _ = self.jobs.send(Job::Load(pos));
        }
    }

    pub fn loading(&self) -> usize {
        self.loading.len()
    }

    /// Whether a save of `pos` is on its way. Two at once could land in either order, and a load
    /// could overtake one: see [`crate::World::save_modified`] and [`crate::World::unload`].
    pub fn is_saving(&self, pos: ChunkPos) -> bool {
        self.saving.contains(&pos)
    }

    /// Queues a save; the caller checks [`Loader::is_saving`] first.
    pub fn save(&mut self, pos: ChunkPos, column: StoredColumn) {
        self.saving.insert(pos);
        let _ = self.jobs.send(Job::Save(pos, column));
    }

    /// The jobs finished since the last call, without waiting.
    pub fn finished(&mut self) -> Vec<Finished> {
        let finished: Vec<_> = self.finished.get_mut().expect("loader results lock").try_iter().collect();
        finished.iter().for_each(|done| self.settle(done));
        finished
    }

    /// Waits for the next job to finish; `None` when none is on its way.
    pub fn wait(&mut self) -> Option<Finished> {
        if self.loading.is_empty() && self.saving.is_empty() {
            return None;
        }
        let done = self.finished.get_mut().expect("loader results lock").recv().ok()?;
        self.settle(&done);
        Some(done)
    }

    fn settle(&mut self, done: &Finished) {
        match done {
            Finished::Loaded(pos, _) => self.loading.remove(pos),
            Finished::Saved { pos, .. } => self.saving.remove(pos),
        };
    }
}
