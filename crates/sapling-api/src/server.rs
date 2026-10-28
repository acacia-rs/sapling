//! What a shard knows of the server around its worlds: the seed, the game mode newcomers get,
//! sending a player to another server, its commands, and script events.

use std::sync::Arc;

use crate::{Commands, Event, GameMode, Player, PlayerId, Sender, ShardCtx};

/// A `/scriptevent <id> <message>`. BDS hands these to its Script API; sapling, which runs no
/// scripts, to listeners, on the tick after the command ran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptEvent {
    /// `namespace:name`, never in the `minecraft` namespace.
    pub id: String,
    pub message: String,
    /// Who ran the command; the server for a function run by the world.
    pub sender: Sender,
}

impl Event for ScriptEvent {
    fn player(&self) -> Option<&Player> {
        self.sender.player()
    }
}

/// Where a backup snapshot of this shard's worlds stands, as BDS's `/save` steps it: hold, query
/// until ready, copy, resume.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveState {
    /// Saving as usual: no hold.
    Idle,
    /// Held, still writing out what was unsaved.
    Saving,
    /// Held and written: these files, cut to these sizes, are a whole copy of the worlds.
    Ready(Vec<SavedFile>),
    /// Held, but writing failed.
    Failed,
}

/// One file of a [`SaveState::Ready`] snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedFile {
    /// From the folder that holds the world folder, `/`-separated: `Bedrock level/db/CURRENT`.
    pub path: String,
    pub size: u64,
}

impl ShardCtx<'_> {
    /// `/save hold`: writes out what this shard's worlds have unsaved and keeps their files as
    /// they are until [`ShardCtx::resume_saves`]; changes wait in memory. False if a hold is on.
    pub fn hold_saves(&mut self) -> bool {
        self.backend.hold_saves()
    }

    /// `/save query`: where the hold stands.
    pub fn save_state(&mut self) -> SaveState {
        self.backend.save_state()
    }

    /// `/save resume`: ends a ready hold, and saving catches up. False unless it was ready.
    pub fn resume_saves(&mut self) -> bool {
        self.backend.resume_saves()
    }

    /// The shard's commands, to run a line as a sender would: `ctx.commands().run(ctx, ..)`.
    pub fn commands(&self) -> Arc<Commands> {
        self.backend.commands()
    }

    /// Raises `event` for listeners on this shard, on its next tick.
    pub fn send_script_event(&mut self, event: ScriptEvent) {
        self.backend.send_script_event(event);
    }

    /// The seed of the world in scope (`RandomSeed`); 0 for a world that has none.
    pub fn seed(&self) -> i64 {
        self.backend.seed()
    }

    /// The game mode players who join this shard for the first time get.
    pub fn default_game_mode(&self) -> GameMode {
        self.backend.default_game_mode()
    }

    /// Changes [`ShardCtx::default_game_mode`] for those who join later; players here keep theirs.
    pub fn set_default_game_mode(&mut self, mode: GameMode) {
        self.backend.set_default_game_mode(mode);
    }

    /// Sends a player's client to the server at `address` and `port`, as `/transfer` does: it
    /// leaves this one, and `on_quit` follows. False for a player who is not in the world.
    pub fn send_to_server(&mut self, player: PlayerId, address: &str, port: u16) -> bool {
        self.backend.send_to_server(player, address, port)
    }
}

/// The server side of [`crate::Backend`].
pub trait ServerBackend {
    fn seed(&self) -> i64;
    fn default_game_mode(&self) -> GameMode;
    fn set_default_game_mode(&mut self, mode: GameMode);
    fn send_to_server(&mut self, player: PlayerId, address: &str, port: u16) -> bool;
    fn commands(&self) -> Arc<Commands> {
        Arc::default()
    }
    fn send_script_event(&mut self, _event: ScriptEvent) {}
    fn hold_saves(&mut self) -> bool {
        false
    }
    fn save_state(&mut self) -> SaveState {
        SaveState::Idle
    }
    fn resume_saves(&mut self) -> bool {
        false
    }
    fn soft_enum(&self, _name: &str) -> Vec<String> {
        Vec::new()
    }
    fn change_soft_enum(&mut self, _name: &str, _change: crate::command::SoftEnumChange) {}
}

#[cfg(test)]
impl ServerBackend for crate::testing::Fake {
    fn seed(&self) -> i64 {
        0
    }
    fn default_game_mode(&self) -> GameMode {
        GameMode::Survival
    }
    fn set_default_game_mode(&mut self, mode: GameMode) {
        self.log.push(format!("default game mode {mode:?}"));
    }
    fn send_to_server(&mut self, player: PlayerId, address: &str, port: u16) -> bool {
        self.log.push(format!("send {} to {address}:{port}", player.0));
        true
    }
}
