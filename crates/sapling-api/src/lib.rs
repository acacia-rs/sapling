//! The plugin-facing API: handler traits called on a shard's thread, with a borrowed [`ShardCtx`]
//! for acting on the shard. Everything here is plain data or a borrowed handle; the ECS behind a
//! shard never shows (docs/DESIGN.md "Plugin API").

mod ctx;
mod player;

pub use ctx::{Backend, ShardCtx};
pub use player::{GameMode, Player, PlayerId, Position};

/// What becomes of a chat message, decided by [`Handler::on_chat`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ChatOutcome {
    /// Everyone on the shard sees it under the sender's name.
    #[default]
    Allow,
    /// Nobody sees it. A handler that formats chat itself cancels and uses [`ShardCtx::broadcast`].
    Cancel,
    /// Sent as [`ChatOutcome::Allow`] would be, with this text instead.
    Replace(String),
}

/// Game events, one method each, all optional. Handlers run on the shard that owns the player, in
/// the order they were registered.
pub trait Handler: Send + 'static {
    /// The player's client has loaded the world and left its loading screen.
    fn on_join(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        let _ = (ctx, player);
    }

    /// A player `on_join` was called for has left, for any reason. They are already gone from
    /// the shard: `ctx` no longer counts them and cannot reach them.
    fn on_quit(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        let _ = (ctx, player);
    }

    /// A player sent a chat message, already cleaned of control characters and cut to length.
    /// The first handler to cancel ends it; a replacement is what later handlers are shown.
    fn on_chat(&mut self, ctx: &mut ShardCtx<'_>, player: &Player, message: &str) -> ChatOutcome {
        let _ = (ctx, player, message);
        ChatOutcome::Allow
    }
}
