//! A backup hold (BDS's `/save hold`): what was unsaved goes to storage, then nothing more is
//! written, so the files stay a whole copy until the hold is released. Written columns stay
//! loaded ([`World::unload`] keeps unsaved ones) and player saves wait in memory meanwhile.

use std::collections::HashSet;
use std::path::PathBuf;

use crate::World;

impl World {
    /// Hands every written column to the I/O threads, then holds later writes.
    pub fn hold_saves(&mut self) {
        self.queue_saves(&HashSet::new());
        self.held = true;
    }

    pub fn is_held(&self) -> bool {
        self.held
    }

    /// Whether what was handed to storage has all landed; [`World::poll`] takes the news.
    pub fn writes_landed(&self) -> bool {
        self.loader.as_ref().is_none_or(|loader| !loader.writing())
    }

    /// Ends a hold: the player saves made during it go to storage, and
    /// [`World::save_modified`] writes columns again.
    pub fn release_saves(&mut self) {
        self.held = false;
        for (id, data) in std::mem::take(&mut self.players.held) {
            self.players.save(self.loader.as_mut(), &id, data);
        }
    }

    /// The folder storage keeps this world's files in, if it keeps them in one.
    pub fn storage_folder(&self) -> Option<PathBuf> {
        self.loader.as_ref().and_then(|loader| loader.source().storage.folder())
    }
}
