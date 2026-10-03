//! The I/O threads between a world and its storage: positions go in, loaded and encoded columns
//! come out, so a shard only ever does a `try_recv`.

use std::collections::HashSet;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

use acacia_world::{BlockRegistry, Dimension};

use crate::ChunkPos;
use crate::column::Column;
use crate::storage::WorldStorage;

/// What every load needs besides the position.
#[derive(Clone)]
pub(crate) struct Source {
    pub storage: Arc<dyn WorldStorage>,
    pub registry: Arc<BlockRegistry>,
    pub dimension: i32,
    pub dim: Dimension,
}

impl Source {
    /// Reads and encodes one column on the calling thread. A column storage does not have, or
    /// cannot read, is void.
    pub fn load(&self, pos: ChunkPos) -> Column {
        let column = match self.storage.load_column(pos, self.dimension) {
            Ok(Some(stored)) => Column::from_chunk(stored.chunk),
            Ok(None) => Column::empty(pos.x, pos.z, self.dim),
            Err(error) => {
                tracing::error!(x = pos.x, z = pos.z, %error, "loading a column failed; serving it as void");
                Column::empty(pos.x, pos.z, self.dim)
            }
        };
        column.warm(&self.registry);
        column
    }
}

pub(crate) struct Loader {
    source: Source,
    requests: Sender<ChunkPos>,
    /// Locked by its one reader only: the lock makes the world `Sync`, as an ECS component must be.
    loaded: Mutex<Receiver<(ChunkPos, Column)>>,
    pending: HashSet<ChunkPos>,
}

impl Loader {
    /// Starts `threads` workers; they end when the loader is dropped.
    pub fn spawn(source: Source, threads: usize) -> Self {
        let (requests, queue) = channel::<ChunkPos>();
        let (deliver, loaded) = channel();
        let queue = Arc::new(Mutex::new(queue));
        for index in 0..threads.max(1) {
            let (source, queue, deliver) = (source.clone(), queue.clone(), deliver.clone());
            let work = move || {
                loop {
                    // The lock covers only the wait for a position, not the load.
                    let next = queue.lock().expect("loader queue lock").recv();
                    let Ok(pos) = next else { return };
                    if deliver.send((pos, source.load(pos))).is_err() {
                        return;
                    }
                }
            };
            std::thread::Builder::new().name(format!("sapling-world-io-{index}")).spawn(work).expect("spawning a world I/O thread");
        }
        Self { source, requests, loaded: Mutex::new(loaded), pending: HashSet::new() }
    }

    pub fn source(&self) -> &Source {
        &self.source
    }

    /// Queues a load unless one is already on its way.
    pub fn request(&mut self, pos: ChunkPos) {
        if self.pending.insert(pos) {
            let _ = self.requests.send(pos);
        }
    }

    pub fn pending(&self) -> usize {
        self.pending.len()
    }

    /// The columns finished since the last call, without waiting.
    pub fn finished(&mut self) -> Vec<(ChunkPos, Column)> {
        let finished: Vec<_> = self.loaded.get_mut().expect("loader results lock").try_iter().collect();
        for (pos, _) in &finished {
            self.pending.remove(pos);
        }
        finished
    }
}
