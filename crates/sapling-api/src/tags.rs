//! Tags on players and entities: what `/tag` adds and a selector's `tag=` tests. They go with a
//! player to other worlds and shards and are saved with players and stored entities, as BDS saves
//! them.

use crate::{EntityId, PlayerId, ShardCtx};

/// What carries tags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tagged {
    Player(PlayerId),
    Entity(EntityId),
}

impl ShardCtx<'_> {
    /// The tags of a player or entity, newest first, as BDS's `/tag list` showed them.
    pub fn tags(&self, of: Tagged) -> Vec<String> {
        self.backend.tags(of)
    }

    /// Puts `tag` on a player or entity; false if it was there or the target is gone.
    pub fn add_tag(&mut self, of: Tagged, tag: &str) -> bool {
        self.backend.add_tag(of, tag)
    }

    /// Takes `tag` off a player or entity; false if it was not there.
    pub fn remove_tag(&mut self, of: Tagged, tag: &str) -> bool {
        self.backend.remove_tag(of, tag)
    }
}

/// The tag side of [`crate::Backend`].
pub trait TagBackend {
    fn tags(&self, of: Tagged) -> Vec<String>;
    fn add_tag(&mut self, of: Tagged, tag: &str) -> bool;
    fn remove_tag(&mut self, of: Tagged, tag: &str) -> bool;
}

#[cfg(test)]
impl TagBackend for crate::testing::Fake {
    fn tags(&self, of: Tagged) -> Vec<String> {
        self.tags.get(&of).map_or_else(Vec::new, |tags| tags.iter().rev().cloned().collect())
    }
    fn add_tag(&mut self, of: Tagged, tag: &str) -> bool {
        let tags = self.tags.entry(of).or_default();
        let new = !tags.iter().any(|held| held == tag);
        if new {
            tags.push(tag.to_owned());
        }
        new
    }
    fn remove_tag(&mut self, of: Tagged, tag: &str) -> bool {
        let Some(tags) = self.tags.get_mut(&of) else { return false };
        let before = tags.len();
        tags.retain(|held| held != tag);
        tags.len() < before
    }
}
