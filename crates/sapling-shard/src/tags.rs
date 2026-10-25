//! What players and entities carry for `/tag` and the scoreboard: their tags and the `UniqueID`
//! the world keeps them under, saved as BDS saves them (`Tags`, `UniqueID`). A player's go with
//! it ([`Marks`]); an entity's tags live in its stored compound.

use bevy_ecs::prelude::*;
use sapling_api::{TagBackend, Tagged};

use crate::entities::{self, Stored};
use crate::handlers::Ctx;

/// A player's `UniqueID` and tags, newest last; carried to other shards and saved with it.
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct Marks {
    pub unique_id: i64,
    pub tags: Vec<String>,
    pub unsaved: bool,
}

impl Marks {
    /// Until its record says otherwise: an id made from its UUID, below zero as BDS's players' are.
    pub fn new(uuid: &[u8; 16]) -> Self {
        let high = u64::from_le_bytes(uuid[..8].try_into().expect("8 bytes"));
        Self { unique_id: -((high >> 1) as i64) - 1, ..Self::default() }
    }

    pub fn restore(&mut self, unique_id: Option<i64>, tags: Vec<String>) {
        self.unique_id = unique_id.unwrap_or(self.unique_id);
        self.tags = tags;
    }
}

/// The `UniqueID` a kept entity (entities.rs [`entities::keep`]) has in its world, for its
/// scores; it stays when the entity stops being kept.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct UniqueId(pub i64);

/// Newest first; an ordering that fit BDS's `/tag list` in both cases seen (unverified).
fn listed(tags: &[String]) -> Vec<String> {
    tags.iter().rev().cloned().collect()
}

impl Ctx<'_> {
    fn stored_tags(&self, id: sapling_api::EntityId) -> Option<Vec<String>> {
        let entity = entities::find(self.world, id)?;
        Some(sapling_world::actor_tags(&self.world.get::<Stored>(entity)?.compound))
    }

    /// Changes an entity's tags in its stored compound; false when it has none or nothing changed.
    fn change_stored(&mut self, id: sapling_api::EntityId, change: impl FnOnce(&mut Vec<String>) -> bool) -> bool {
        let Some(entity) = entities::find(self.world, id) else { return false };
        let Some(mut stored) = self.world.get_mut::<Stored>(entity) else { return false };
        let mut tags = sapling_world::actor_tags(&stored.compound);
        if !change(&mut tags) {
            return false;
        }
        match sapling_world::tagged_actor(&stored.compound, &tags) {
            Ok(compound) => stored.compound = compound,
            Err(error) => {
                tracing::warn!(%error, "an entity's compound is unreadable: its tags are not kept");
                return false;
            }
        }
        true
    }

    fn change_tags(&mut self, of: Tagged, change: impl FnOnce(&mut Vec<String>) -> bool) -> bool {
        match of {
            Tagged::Player(player) => {
                let Some(mut marks) = self.entity(player).and_then(|entity| self.world.get_mut::<Marks>(entity)) else { return false };
                let changed = change(&mut marks.tags);
                marks.unsaved |= changed;
                changed
            }
            Tagged::Entity(id) => self.change_stored(id, change),
        }
    }
}

impl TagBackend for Ctx<'_> {
    fn tags(&self, of: Tagged) -> Vec<String> {
        match of {
            Tagged::Player(player) => self.entity(player).and_then(|entity| self.world.get::<Marks>(entity)).map_or_else(Vec::new, |marks| listed(&marks.tags)),
            Tagged::Entity(id) => self.stored_tags(id).map_or_else(Vec::new, |tags| listed(&tags)),
        }
    }

    fn add_tag(&mut self, of: Tagged, tag: &str) -> bool {
        self.change_tags(of, |tags| {
            let new = !tags.iter().any(|held| held == tag);
            if new {
                tags.push(tag.to_owned());
            }
            new
        })
    }

    fn remove_tag(&mut self, of: Tagged, tag: &str) -> bool {
        self.change_tags(of, |tags| {
            let before = tags.len();
            tags.retain(|held| held != tag);
            tags.len() < before
        })
    }
}
