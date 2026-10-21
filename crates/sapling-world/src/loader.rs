//! The I/O threads between a world and its storage: positions go in and loaded, encoded columns
//! come out; written columns go in and are saved. A shard only ever does a `try_recv`.
//! A storage that panics costs the job, not the thread: a world waits for every job it queued.

use std::collections::HashSet;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};

use acacia_world::{BlockRegistry, Dimension};
use sapling_leveldb::PlayerData;

use crate::ChunkPos;
use crate::column::Column;
use crate::load_queue::{JobQueue, Next};
use crate::players::PlayerLoad;
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

    fn load_player(&self, id: &str) -> PlayerLoad {
        match catch_unwind(AssertUnwindSafe(|| self.storage.load_player(id))) {
            Ok(Ok(Some(data))) => return PlayerLoad::Stored(data),
            Ok(Ok(None)) => return PlayerLoad::New,
            Ok(Err(error)) => tracing::error!(player = id, %error, "loading a player failed; they start empty and are not saved"),
            Err(_) => tracing::error!(player = id, "storage panicked loading a player; they start empty and are not saved"),
        }
        PlayerLoad::Unreadable
    }

    fn save_player(&self, id: &str, data: &PlayerData) {
        match catch_unwind(AssertUnwindSafe(|| self.storage.store_player(id, data))) {
            Ok(Ok(())) => {}
            Ok(Err(error)) => tracing::error!(player = id, %error, "saving a player failed"),
            Err(_) => tracing::error!(player = id, "storage panicked saving a player"),
        }
    }
}

/// What the I/O threads do besides loads, which load_queue.rs keeps apart.
enum Job {
    Save(ChunkPos, StoredColumn),
    LoadPlayer(String),
    SavePlayer(String, PlayerData),
}

/// One job the I/O threads are done with.
pub(crate) enum Finished {
    Loaded(ChunkPos, Column),
    Saved { pos: ChunkPos, ok: bool },
    PlayerLoaded(String, PlayerLoad),
    PlayerSaved(String, PlayerData),
}

pub(crate) struct Loader {
    source: Source,
    jobs: Arc<JobQueue<Job>>,
    /// Locked by its one reader only: the lock makes the world `Sync`, as an ECS component must be.
    finished: Mutex<Receiver<Finished>>,
    loading: HashSet<ChunkPos>,
    saving: HashSet<ChunkPos>,
    /// Player loads and saves on their way.
    player_jobs: usize,
}

impl Loader {
    pub fn storage(&self) -> &dyn WorldStorage {
        &*self.source.storage
    }

    /// Starts `threads` workers; they end when the loader is dropped and its queue is empty.
    pub fn spawn(source: Source, threads: usize) -> Self {
        let threads = threads.max(1);
        let jobs = Arc::new(JobQueue::new(threads));
        let (deliver, finished) = channel();
        for index in 0..threads {
            let (source, queue, deliver) = (source.clone(), jobs.clone(), deliver.clone());
            let work = move || {
                while let Some(next) = queue.next(index) {
                    let done = match next {
                        Next::Load(pos) => Finished::Loaded(pos, source.load(pos)),
                        Next::Other(Job::Save(pos, chunk)) => Finished::Saved { pos, ok: source.save(pos, &chunk) },
                        Next::Other(Job::LoadPlayer(id)) => {
                            let load = source.load_player(&id);
                            Finished::PlayerLoaded(id, load)
                        }
                        Next::Other(Job::SavePlayer(id, data)) => {
                            source.save_player(&id, &data);
                            Finished::PlayerSaved(id, data)
                        }
                    };
                    // Nobody listens once the world is gone, but the saves still queued must happen.
                    let _ = deliver.send(done);
                }
            };
            std::thread::Builder::new().name(format!("sapling-world-io-{index}")).spawn(work).expect("spawning a world I/O thread");
        }
        Self { source, jobs, finished: Mutex::new(finished), loading: HashSet::new(), saving: HashSet::new(), player_jobs: 0 }
    }

    pub fn source(&self) -> &Source {
        &self.source
    }

    /// Queues a load unless one is already on its way; an urgent one goes first (load_queue.rs).
    pub fn request(&mut self, pos: ChunkPos, urgent: bool) {
        if self.loading.insert(pos) {
            self.jobs.push_load(pos, urgent);
        } else if urgent {
            self.jobs.promote(pos);
        }
    }

    /// Drops the queued loads `keep` rejects, so a later request queues them again; returns how
    /// many. One a worker already took still arrives.
    pub fn cancel(&mut self, keep: impl Fn(ChunkPos) -> bool) -> usize {
        let cancelled = self.jobs.cancel_loads(keep);
        for pos in &cancelled {
            self.loading.remove(pos);
        }
        cancelled.len()
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
        self.jobs.push_other(Job::Save(pos, column));
    }

    pub fn load_player(&mut self, id: String) {
        self.player_jobs += 1;
        self.jobs.push_other(Job::LoadPlayer(id));
    }

    /// Queues a save; players.rs keeps two of one player from being on their way at once.
    pub fn save_player(&mut self, id: String, data: PlayerData) {
        self.player_jobs += 1;
        self.jobs.push_other(Job::SavePlayer(id, data));
    }

    /// The jobs finished since the last call, without waiting.
    pub fn finished(&mut self) -> Vec<Finished> {
        let finished: Vec<_> = self.finished.get_mut().expect("loader results lock").try_iter().collect();
        finished.iter().for_each(|done| self.settle(done));
        finished
    }

    /// Waits for the next job to finish; `None` when none is on its way.
    pub fn wait(&mut self) -> Option<Finished> {
        if self.loading.is_empty() && self.saving.is_empty() && self.player_jobs == 0 {
            return None;
        }
        let done = self.finished.get_mut().expect("loader results lock").recv().ok()?;
        self.settle(&done);
        Some(done)
    }

    fn settle(&mut self, done: &Finished) {
        match done {
            Finished::Loaded(pos, _) => drop(self.loading.remove(pos)),
            Finished::Saved { pos, .. } => drop(self.saving.remove(pos)),
            Finished::PlayerLoaded(..) | Finished::PlayerSaved(..) => self.player_jobs -= 1,
        }
    }
}

impl Drop for Loader {
    fn drop(&mut self) {
        self.jobs.close();
    }
}
