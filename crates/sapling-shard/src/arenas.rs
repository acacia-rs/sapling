//! Worlds made at runtime from a template (docs/DESIGN.md "Arenas").
// TODO: part B of step 5; until then no template is known and every call refuses.

use bevy_ecs::prelude::*;

pub(crate) fn create(_world: &mut World, _name: &str, _template: &str) -> bool {
    false
}

pub(crate) fn remove(_world: &mut World, _name: &str) -> bool {
    false
}

pub(crate) fn reset(_world: &mut World, _name: &str) -> bool {
    false
}
