//! The plugin-facing API: handler traits called on a shard's thread, with a borrowed [`ShardCtx`]
//! for acting on the shard. Everything here is plain data or a borrowed handle; the ECS behind a
//! shard never shows (docs/DESIGN.md "Plugin API").

pub mod command;
mod ctx;
mod event;
pub mod form;
mod hud;
mod player;
#[cfg(test)]
mod testing;
#[cfg(test)]
mod tests;

pub use command::{Command, Commands, Output, Sender};
pub use ctx::{Backend, FormCallback, ShardCtx, State};
pub use event::{Cancellable, ChatEvent, CommandEvent, Priority};
pub use form::{Custom, Form, FormResponse, Menu, Modal};
pub use hud::{BossBar, BossBarColour, Sidebar, Title};
pub use player::{Difficulty, GameMode, Permission, Player, PlayerId, Position};
pub use sapling_macros::{Command, CommandEnum, Form};

/// Game events, one method each, all optional. Handlers run on the shard that owns the player, in
/// [`Priority`] order.
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

    /// A player sent a chat message. What is left of it after the handlers goes to everyone.
    fn on_chat(&mut self, ctx: &mut ShardCtx<'_>, player: &Player, event: &mut ChatEvent) {
        let _ = (ctx, player, event);
    }

    /// A player sent a command, registered or not. Cancelled, it is not run and gets no answer.
    fn on_command(&mut self, ctx: &mut ShardCtx<'_>, player: &Player, event: &mut CommandEvent) {
        let _ = (ctx, player, event);
    }

    /// Once per tick, after the events of the tick.
    fn on_tick(&mut self, ctx: &mut ShardCtx<'_>) {
        let _ = ctx;
    }
}
