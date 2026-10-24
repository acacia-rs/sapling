//! Tags on players and entities: what `/tag` adds and a selector's `tag=` tests.

use std::collections::{BTreeSet, HashMap};

use crate::{EntityId, PlayerId, ShardCtx};

/// What carries tags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tagged {
    Player(PlayerId),
    Entity(EntityId),
}

// TODO: kept in the shard's state: not saved, not taken along to another shard, and an entity's
// stay after it is gone.
#[derive(Default)]
struct Tags(HashMap<Tagged, BTreeSet<String>>);

impl ShardCtx<'_> {
    /// The tags of a player or entity, sorted.
    pub fn tags(&mut self, of: Tagged) -> Vec<String> {
        self.state::<Tags>().0.get(&of).map_or_else(Vec::new, |tags| tags.iter().cloned().collect())
    }

    /// Puts `tag` on a player or entity; false if it was there.
    pub fn add_tag(&mut self, of: Tagged, tag: &str) -> bool {
        self.state::<Tags>().0.entry(of).or_default().insert(tag.to_owned())
    }

    /// Takes `tag` off a player or entity; false if it was not there.
    pub fn remove_tag(&mut self, of: Tagged, tag: &str) -> bool {
        self.state::<Tags>().0.get_mut(&of).is_some_and(|tags| tags.remove(tag))
    }
}
