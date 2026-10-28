//! What a world keeps of its players between visits, through the same I/O threads as its columns.

use std::collections::HashMap;

use sapling_leveldb::PlayerData;

use crate::World;
use crate::loader::Loader;

/// What storage had for a player who came.
#[derive(Debug, Clone, PartialEq)]
pub enum PlayerLoad {
    /// Nothing: a first visit, or a world without storage.
    New,
    Stored(PlayerData),
    /// Storage holds something it could not read. Nothing must be saved over it.
    Unreadable,
}

impl World {
    /// Asks storage for what it kept of player `id` (its UUID as text); [`World::take_players`]
    /// has the answer after a later [`World::poll`], or at once in a world without storage.
    pub fn request_player(&mut self, id: &str) {
        self.players.request(self.loader.as_mut(), id);
    }

    pub fn take_players(&mut self) -> Vec<(String, PlayerLoad)> {
        self.players.take_arrived()
    }

    /// Hands a player's items to the I/O threads; [`World::save_and_wait`] waits for them too.
    pub fn save_player(&mut self, id: &str, data: PlayerData) {
        match self.held {
            true => drop(self.players.held.insert(id.to_owned(), data)),
            false => self.players.save(self.loader.as_mut(), id, data),
        }
    }
}

#[derive(Default)]
pub(crate) struct Players {
    /// The newest save of each player with one on its way: a player back before storage has
    /// them is given this.
    saving: HashMap<String, PlayerData>,
    arrived: Vec<(String, PlayerLoad)>,
    /// Saves made during a backup hold, handed to storage when it ends.
    pub(crate) held: HashMap<String, PlayerData>,
}

impl Players {
    /// Asks for what is kept of player `id`; the answer comes from [`Players::take_arrived`].
    pub fn request(&mut self, loader: Option<&mut Loader>, id: &str) {
        match (self.held.get(id).or_else(|| self.saving.get(id)), loader) {
            (Some(data), _) => self.arrived.push((id.to_string(), PlayerLoad::Stored(data.clone()))),
            (None, Some(loader)) => loader.load_player(id.to_string()),
            (None, None) => self.arrived.push((id.to_string(), PlayerLoad::New)),
        }
    }

    /// Hands a player to storage. One save per player is on its way at a time, so two cannot
    /// land in the wrong order; a later one waits for [`Players::saved`].
    pub fn save(&mut self, loader: Option<&mut Loader>, id: &str, data: PlayerData) {
        let Some(loader) = loader else { return };
        if self.saving.insert(id.to_string(), data.clone()).is_none() {
            loader.save_player(id.to_string(), data);
        }
    }

    pub fn loaded(&mut self, id: String, load: PlayerLoad) {
        self.arrived.push((id, load));
    }

    /// A save of `written` ended; if the player was saved again meanwhile, that goes next.
    pub fn saved(&mut self, loader: &mut Loader, id: String, written: &PlayerData) {
        match self.saving.get(&id) {
            Some(newest) if newest != written => loader.save_player(id, newest.clone()),
            _ => drop(self.saving.remove(&id)),
        }
    }

    pub fn take_arrived(&mut self) -> Vec<(String, PlayerLoad)> {
        std::mem::take(&mut self.arrived)
    }
}
