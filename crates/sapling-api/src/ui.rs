//! The rest of what a plugin shows one player: popups and tips over the hotbar, and the backend
//! side of dialogues, debug shapes, labels, hidden entities, floating text and further boss bars
//! (docs/research/ui-events.md).

use crate::{BossBar, DebugShape, Dialogue, DialogueCallback, EntityId, PlayerId, ShardCtx};

/// A short line over the hotbar, each in its own place on the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudLine {
    /// Over the hotbar, above the action bar.
    Popup,
    /// Higher up than a popup.
    Tip,
    /// Where a jukebox shows the disc it plays, in its rainbow colours.
    JukeboxPopup,
}

/// A player or an entity, as the calls that take either name it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Actor {
    Player(PlayerId),
    Entity(EntityId),
}

impl From<PlayerId> for Actor {
    fn from(player: PlayerId) -> Self {
        Self::Player(player)
    }
}

impl From<EntityId> for Actor {
    fn from(entity: EntityId) -> Self {
        Self::Entity(entity)
    }
}

/// Which of the two lines over an entity's head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Label {
    /// The name, shown at any distance the entity is seen at.
    NameTag,
    /// The smaller line under the name.
    ScoreTag,
}

impl ShardCtx<'_> {
    /// Shows `text` over a player's hotbar for a few seconds.
    pub fn send_popup(&mut self, player: PlayerId, text: &str) {
        self.backend.send_hud_line(player, HudLine::Popup, text);
    }

    /// Shows `text` above where a popup goes for a few seconds.
    pub fn send_tip(&mut self, player: PlayerId, text: &str) {
        self.backend.send_hud_line(player, HudLine::Tip, text);
    }

    /// Shows `text` where a jukebox names its disc.
    pub fn send_jukebox_popup(&mut self, player: PlayerId, text: &str) {
        self.backend.send_hud_line(player, HudLine::JukeboxPopup, text);
    }
}

/// The dialogue, debug shape and display side of [`crate::Backend`].
pub trait UiBackend {
    fn send_hud_line(&mut self, _player: PlayerId, _line: HudLine, _text: &str) {}
    fn set_label(&mut self, _actor: Actor, _label: Label, _text: Option<&str>) -> bool {
        false
    }
    fn set_hidden(&mut self, _viewer: PlayerId, _actor: Actor, _hidden: bool) -> bool {
        false
    }
    fn add_floating_text(&mut self, _at: [f32; 3], _text: &str) -> Option<EntityId> {
        None
    }
    fn set_floating_text(&mut self, _id: EntityId, _text: &str) -> bool {
        false
    }
    fn set_extra_boss_bar(&mut self, _player: PlayerId, _key: u32, _bar: Option<&BossBar>) {}
    fn debug_shape(&mut self, _player: PlayerId, _id: u64, _shape: Option<&DebugShape>) {}
    fn send_dialogue(&mut self, _player: PlayerId, _npc: Actor, _dialogue: &Dialogue, _on_press: DialogueCallback) -> bool {
        false
    }
    fn close_dialogue(&mut self, _player: PlayerId) {}
}

#[cfg(test)]
impl UiBackend for crate::testing::Fake {}
