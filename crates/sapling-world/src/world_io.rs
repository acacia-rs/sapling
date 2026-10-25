//! A world's side of its I/O threads (loader.rs): requests, what they finished, saves and unloading.

use std::collections::HashSet;
use std::collections::hash_map::Entry;

use crate::column::Column;
use crate::light::ColumnLight;
use crate::loader::{Finished, Loader};
use crate::{ChunkPos, World};

impl World {
    /// Takes what the I/O threads finished; returns how many columns they loaded. Call once a tick.
    pub fn poll(&mut self) -> usize {
        let Some(loader) = &mut self.loader else { return 0 };
        let finished = loader.finished();
        finished.into_iter().map(|done| self.accept(done)).filter(|&loaded| loaded).count()
    }

    /// Applies one finished job; true for a loaded column. A failed save leaves its column unsaved.
    fn accept(&mut self, done: Finished) -> bool {
        match done {
            Finished::Loaded(pos, column, light) => {
                self.insert_loaded(pos, column, light);
                true
            }
            Finished::Saved { pos, ok } => {
                if let (false, Some(column)) = (ok, self.columns.get_mut(&pos)) {
                    column.mark_modified();
                }
                false
            }
            Finished::PlayerLoaded(id, load) => {
                self.players.loaded(id, load);
                false
            }
            Finished::PlayerSaved(id, written) => {
                if let Some(loader) = &mut self.loader {
                    self.players.saved(loader, id, &written);
                }
                false
            }
        }
    }

    /// Hands every column written since its last save to the I/O threads; returns how many.
    /// A column whose previous save is still on its way waits for the next call.
    pub fn save_modified(&mut self) -> usize {
        self.queue_saves(&HashSet::new())
    }

    fn insert_loaded(&mut self, pos: ChunkPos, column: Column, light: ColumnLight) {
        if let Entry::Vacant(slot) = self.columns.entry(pos) {
            slot.insert(column);
            self.arrived.push(pos);
            self.light.arrived(pos, light);
        }
    }

    /// Saves every written column, waits for the I/O threads and flushes storage with the
    /// world's [`crate::Level`]; returns how many columns could not be saved. Blocks on disk: for a
    /// world that is being closed, never for a tick.
    pub fn save_and_wait(&mut self) -> usize {
        let mut failed = HashSet::new();
        loop {
            self.queue_saves(&failed);
            let Some(done) = self.loader.as_mut().and_then(Loader::wait) else { break };
            if let Finished::Saved { pos, ok: false } = &done {
                failed.insert(*pos);
            }
            self.accept(done);
        }
        let flushed = self.loader.as_ref().map(|loader| match self.dimension {
            crate::OVERWORLD => loader.source().storage.flush(&self.level),
            _ => loader.source().storage.sync(),
        });
        if let Some(Err(error)) = flushed {
            tracing::error!(%error, "flushing storage failed");
        }
        failed.len()
    }

    fn queue_saves(&mut self, skip: &HashSet<ChunkPos>) -> usize {
        let Some(loader) = &mut self.loader else { return 0 };
        let mut queued = 0;
        for (pos, column) in &mut self.columns {
            if column.is_modified() && !loader.is_saving(*pos) && !skip.contains(pos) {
                loader.save(*pos, column.stored());
                column.mark_saved();
                queued += 1;
            }
        }
        queued
    }

    /// Whether the column can be served now; if not, its load is queued (once).
    pub fn request(&mut self, pos: ChunkPos) -> bool {
        self.request_load(pos, false)
    }

    /// [`World::request`] for a column someone waits on, such as a joining player's spawn: its
    /// load goes ahead of the others, from whichever I/O thread is free first.
    pub fn request_urgent(&mut self, pos: ChunkPos) -> bool {
        self.request_load(pos, true)
    }

    fn request_load(&mut self, pos: ChunkPos, urgent: bool) -> bool {
        let loaded = self.is_loaded(pos);
        if let (false, Some(loader)) = (loaded, &mut self.loader) {
            loader.request(pos, urgent);
        }
        loaded
    }

    /// Loads a column on the calling thread: for opening a world (folder.rs), not for a shard.
    pub(crate) fn load_now(&mut self, pos: ChunkPos) {
        if let (false, Some(loader)) = (self.is_loaded(pos), &self.loader) {
            let (column, light) = loader.source().load(pos);
            self.insert_loaded(pos, column, light);
        }
    }

    /// Drops the loads still queued for columns `keep` rejects; returns how many. See
    /// `Loader::cancel`.
    pub fn cancel_loads(&mut self, keep: impl Fn(ChunkPos) -> bool) -> usize {
        self.loader.as_mut().map_or(0, |loader| loader.cancel(keep))
    }

    /// Columns held in memory, and loads on their way.
    pub fn loaded_columns(&self) -> (usize, usize) {
        (self.columns.len(), self.loader.as_ref().map_or(0, Loader::loading))
    }

    /// Drops the loaded columns `keep` rejects, except those storage does not hold yet: unsaved
    /// ones, and ones whose save is on its way (a reload could read the column before the save).
    pub fn unload(&mut self, keep: impl Fn(ChunkPos) -> bool) {
        if let Some(loader) = &self.loader {
            self.columns.retain(|pos, column| column.is_modified() || loader.is_saving(*pos) || keep(*pos));
            let columns = &self.columns;
            self.light.retain(|pos| columns.contains_key(&pos));
        }
    }
}
