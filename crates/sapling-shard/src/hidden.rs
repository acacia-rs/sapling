//! What plugins hid from one player: the visibility diffs (visibility.rs, mobs.rs) leave it out of
//! that player's view, so it is removed and announced again as any entity leaving and entering.

use std::collections::HashSet;

use bevy_ecs::prelude::*;

use crate::mobs::SeesMobs;

/// The players and entities this player must not see.
#[derive(Component, Default)]
pub(crate) struct Hidden(HashSet<Entity>);

impl Hidden {
    pub fn hides(hidden: Option<&Self>, entity: Entity) -> bool {
        hidden.is_some_and(|hidden| hidden.0.contains(&entity))
    }
}

pub(crate) fn set(world: &mut World, viewer: Entity, target: Entity, hidden: bool) {
    if world.get::<Hidden>(viewer).is_none() {
        world.entity_mut(viewer).insert(Hidden::default());
    }
    let mut set = world.get_mut::<Hidden>(viewer).expect("just inserted");
    if hidden { set.0.insert(target) } else { set.0.remove(&target) };
    if let Some(mut sees) = world.get_mut::<SeesMobs>(viewer) {
        sees.recheck();
    }
}
