//! How entities look to players, set by plugins: the lines over their heads, who sees them at
//! all, and text that floats on its own.

use crate::{Actor, EntityId, Label, PlayerId, ShardCtx};

impl ShardCtx<'_> {
    /// Puts `text` over a player's or an entity's head for everyone who sees it, always shown;
    /// `None` gives it back its own name (a player's, or the one a name tag gave a mob). False
    /// for one that is not there.
    pub fn set_name_tag(&mut self, actor: impl Into<Actor>, text: Option<&str>) -> bool {
        self.backend.set_label(actor.into(), Label::NameTag, text)
    }

    /// Puts `text` under the name of a player or an entity; `None` takes it off. False for one
    /// that is not there.
    pub fn set_score_tag(&mut self, actor: impl Into<Actor>, text: Option<&str>) -> bool {
        self.backend.set_label(actor.into(), Label::ScoreTag, text)
    }

    /// Takes a player or an entity off one player's screen until [`ShardCtx::show_to`]; others
    /// still see it. Kept while both are on the shard. False where either is not there.
    pub fn hide_from(&mut self, viewer: PlayerId, actor: impl Into<Actor>) -> bool {
        self.backend.set_hidden(viewer, actor.into(), true)
    }

    /// Shows `viewer` again what [`ShardCtx::hide_from`] hid, once it is in their view.
    pub fn show_to(&mut self, viewer: PlayerId, actor: impl Into<Actor>) -> bool {
        self.backend.set_hidden(viewer, actor.into(), false)
    }

    /// Floats `text` at `at` in the world of the event, for everyone who sees the place: an
    /// entity of its own with nothing but a name, which does not move. Taken away with
    /// [`ShardCtx::remove_entity`]; it is not saved. `None` where the world is not loaded.
    pub fn add_floating_text(&mut self, at: [f32; 3], text: &str) -> Option<EntityId> {
        self.backend.add_floating_text(at, text)
    }

    /// Changes what floating text says. False for an id that is no floating text.
    pub fn set_floating_text(&mut self, id: EntityId, text: &str) -> bool {
        self.backend.set_floating_text(id, text)
    }
}
