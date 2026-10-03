//! The plugin-facing API: handler traits called on a shard's thread, with a borrowed [`ShardCtx`]
//! for acting on the shard. Everything here is plain data or a borrowed handle; the ECS behind a
//! shard never shows (docs/DESIGN.md "Plugin API").

mod ctx;
mod player;

pub use ctx::{Backend, ShardCtx};
pub use player::{GameMode, Player, PlayerId};

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
}
