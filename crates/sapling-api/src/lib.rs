//! The plugin-facing API: [`Plugin`]s add typed listeners and tasks that run on a shard's thread
//! with a borrowed [`ShardCtx`] for acting on the shard. Everything here is plain data or a
//! borrowed handle; the ECS behind a shard never shows (docs/DESIGN.md "Plugin API").

mod block;
mod chat;
mod combat;
pub mod command;
mod ctx;
mod dragon;
mod effect;
mod entity;
mod experience;
mod event;
pub mod form;
mod health;
mod hud;
mod item;
mod light;
mod listen;
mod movement;
mod player;
mod plugin;
mod scoreboard;
mod sound;
mod tags;
#[cfg(test)]
mod testing;
mod weather;
#[cfg(test)]
mod tests;
mod world;
mod world_event;

pub use block::{Block, BlockPos, BreakEvent, Face, InteractEvent, PlaceEvent};
pub use chat::ChatBackend;
pub use movement::{MovementBackend, MovementPolicy};
pub use combat::{Attacker, DamageEvent, DeathEvent, DeathMessage, RespawnEvent};
pub use command::{Command, Commands, Output, Sender};
pub use ctx::{Backend, FormCallback, ShardCtx, State};
pub use dragon::{DragonFight, DragonKilledEvent, DragonRespawnEvent};
pub use effect::{Effect, EffectBackend};
pub use experience::ExperienceBackend;
pub use weather::{Weather, WeatherBackend};
pub use light::{Light, LightBackend};
pub use entity::{EntityBackend, EntityId, EntityInfo};
pub use world::{RuleValue, WorldBackend};
pub use world_event::{BlockGrowEvent, BlockSpreadEvent, EntitySpawnEvent, ExplosionEvent, SpawnReason};
pub use event::{
    ChatEvent, CommandEvent, EntityAttackEvent, EntityInteractEvent, ItemUseEvent, JoinEvent, MoveEvent, Priority, QuitEvent, Target, TickEvent, TransferFailedEvent,
    WorldChangeEvent,
};
pub use listen::{Event, Listen, ListenerChange, ListenerId, Listeners, Scope};
pub use plugin::{Plugin, PluginBuilder, Scoped};
pub use health::{DamageCause, HealthBackend};
pub use form::{Custom, Form, FormResponse, Menu, Modal};
pub use hud::{BossBar, BossBarColour, Sidebar, Title, TitleAction, TitleBackend};
pub use sound::{Sound, SoundBackend};
pub use item::{DropEvent, Item, Slot};
pub use player::{Difficulty, GameMode, Permission, Player, PlayerId, Position};
pub use tags::{TagBackend, Tagged};
pub use scoreboard::{Display, DisplaySlot, Objective, ScoreHolder, ScoreOperation, ScoreboardBackend, SortOrder};
pub use sapling_macros::{Command, CommandEnum, Form};

/// The game's events, one method each, all optional: a plugin as one value. Each method is filed
/// as a listener for its event at the handler's priority ([`PluginBuilder::handler`]); the events'
/// docs say when they come.
pub trait Handler: Send + 'static {
    /// The player's client has loaded the world and left its loading screen.
    fn on_join(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        let _ = (ctx, player);
    }

    /// A player `on_join` was called for has left the server, for any reason. They are already
    /// gone from the shard: `ctx` no longer counts them and cannot reach them. Called on the
    /// shard they were on last, which need not be the one `on_join` ran on.
    fn on_quit(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        let _ = (ctx, player);
    }

    /// A player was moved from world `from` to world `to` ([`ShardCtx::transfer`], or a world
    /// that was removed). Called on the shard of `to`, with the player already there; `from`
    /// may be on another shard.
    fn on_world_change(&mut self, ctx: &mut ShardCtx<'_>, player: &Player, from: &str, to: &str) {
        let _ = (ctx, player, from, to);
    }

    /// A transfer [`ShardCtx::transfer`] accepted could not be made: the shard of world `to`
    /// refused or did not answer. The player is where they were.
    fn on_transfer_failed(&mut self, ctx: &mut ShardCtx<'_>, player: &Player, to: &str) {
        let _ = (ctx, player, to);
    }

    /// A player sent a chat message. What is left of it after the handlers goes to everyone.
    fn on_chat(&mut self, ctx: &mut ShardCtx<'_>, player: &Player, event: &mut ChatEvent) {
        let _ = (ctx, player, event);
    }

    /// A player sent a command, registered or not. Cancelled, it is not run and gets no answer.
    fn on_command(&mut self, ctx: &mut ShardCtx<'_>, player: &Player, event: &mut CommandEvent) {
        let _ = (ctx, player, event);
    }

    /// A player finished breaking a block (in creative mode: hit it).
    fn on_break(&mut self, ctx: &mut ShardCtx<'_>, player: &Player, event: &mut BreakEvent) {
        let _ = (ctx, player, event);
    }

    /// A player clicked a block with a block in hand.
    fn on_place(&mut self, ctx: &mut ShardCtx<'_>, player: &Player, event: &mut PlaceEvent) {
        let _ = (ctx, player, event);
    }

    /// A player clicked a block that is used rather than built on, or clicked with no block in hand.
    fn on_interact(&mut self, ctx: &mut ShardCtx<'_>, player: &Player, event: &mut InteractEvent) {
        let _ = (ctx, player, event);
    }

    /// A player threw a stack out of their inventory; it becomes an item lying in the world.
    fn on_drop(&mut self, ctx: &mut ShardCtx<'_>, player: &Player, event: &mut DropEvent) {
        let _ = (ctx, player, event);
    }

    /// Once per tick, after the events of the tick.
    fn on_tick(&mut self, ctx: &mut ShardCtx<'_>) {
        let _ = ctx;
    }
}
