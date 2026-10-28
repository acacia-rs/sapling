//! NPC dialogue: the talk screen an entity opens on one player's client, with up to six buttons
//! whose presses come back to a callback, as forms' answers do (docs/research/ui-events.md).

use crate::{Actor, Player, PlayerId, ShardCtx};

/// Called on the shard of the player it was opened for: `Some(i)` for each press of button `i`,
/// then `None` once, when the screen is closed (by the player, another dialogue, or
/// [`ShardCtx::close_dialogue`]). Never if the player leaves first.
pub type DialogueCallback = Box<dyn FnMut(&mut ShardCtx<'_>, &Player, Option<usize>) + Send>;

/// One button: its text, and the commands a press runs before the callback hears it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DialogueButton {
    pub text: String,
    /// Run in order as the NPC, at it, with operator rights; `@initiator` is who pressed.
    pub commands: Vec<String>,
}

/// The words of a dialogue: the NPC's name over a body of text, and the buttons under it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Dialogue {
    pub title: String,
    pub body: String,
    pub buttons: Vec<DialogueButton>,
}

impl Dialogue {
    /// The client shows no more.
    pub const MAX_BUTTONS: usize = 6;

    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into(), ..Self::default() }
    }

    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = body.into();
        self
    }

    /// Adds a button; past [`Dialogue::MAX_BUTTONS`] they are dropped.
    pub fn button(self, text: impl Into<String>) -> Self {
        self.button_running(text, Vec::<String>::new())
    }

    /// Adds a button whose press runs `commands` (see [`DialogueButton::commands`]).
    pub fn button_running(mut self, text: impl Into<String>, commands: impl IntoIterator<Item = impl Into<String>>) -> Self {
        if self.buttons.len() < Self::MAX_BUTTONS {
            self.buttons.push(DialogueButton { text: text.into(), commands: commands.into_iter().map(Into::into).collect() });
        }
        self
    }

    /// The buttons as the client reads them from NpcDialogue's action JSON: command buttons
    /// pressed by a click, whose commands stay on the server.
    pub fn actions_json(&self) -> String {
        let buttons: Vec<_> = self.buttons.iter().map(|button| serde_json::json!({"button_name": button.text, "text": "", "mode": 0, "type": 1})).collect();
        serde_json::Value::Array(buttons).to_string()
    }
}

impl ShardCtx<'_> {
    /// Opens `dialogue` on a player's screen, spoken by `npc`, a player or an entity the player
    /// sees. False, with nothing sent, where either is not there. One dialogue at a time: a new
    /// one closes the last.
    pub fn send_dialogue(&mut self, player: PlayerId, npc: impl Into<Actor>, dialogue: &Dialogue, on_press: impl FnMut(&mut ShardCtx<'_>, &Player, Option<usize>) + Send + 'static) -> bool {
        self.backend.send_dialogue(player, npc.into(), dialogue, Box::new(on_press))
    }

    /// Closes the dialogue a player has open, if any; its callback gets `None`.
    pub fn close_dialogue(&mut self, player: PlayerId) {
        self.backend.close_dialogue(player);
    }
}
