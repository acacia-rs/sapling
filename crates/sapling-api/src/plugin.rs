//! A plugin: what it adds to each shard, and how a running listener adds more.

use std::sync::{Arc, Mutex};

use crate::listen::Run;
use crate::{
    BreakEvent, ChatEvent, Command, CommandEvent, Commands, DropEvent, Event, Handler, InteractEvent, JoinEvent, Listen, ListenerChange, ListenerId, Listeners,
    PlaceEvent, PlayerId, Priority, QuitEvent, Scope, ShardCtx, TickEvent, TransferFailedEvent, WorldChangeEvent,
};

/// What a server adds to each of its shards. [`Plugin::build`] runs once per shard, so what a
/// plugin keeps per shard lives in its closures or [`ShardCtx::state`], and what it shares between
/// shards in an `Arc` it holds.
pub trait Plugin: Send + Sync + 'static {
    fn build(&self, plugin: &mut PluginBuilder<'_>);

    /// Adds custom items and blocks, once for the whole server, before any shard is built.
    fn content(&self, content: &mut crate::Content) {
        let _ = content;
    }
}

/// What [`Plugin::build`] adds to: commands, and listeners and tasks for the whole shard.
pub struct PluginBuilder<'a> {
    listeners: &'a mut Listeners,
    commands: &'a mut Commands,
}

impl<'a> PluginBuilder<'a> {
    pub fn new(listeners: &'a mut Listeners, commands: &'a mut Commands) -> Self {
        Self { listeners, commands }
    }

    /// Adds a command: clients that may use it see its syntax and get completions.
    pub fn command<C: Command>(&mut self) -> &mut Self {
        self.commands.register::<C>();
        self
    }

    /// Files each method of `handler` as a listener at `priority`.
    pub fn handler(&mut self, priority: Priority, handler: Box<dyn Handler>) -> &mut Self {
        let shared: Shared = Arc::new(Mutex::new(handler));
        let mut file = |change| self.listeners.apply(change);
        file(adapt::<JoinEvent>(priority, &shared, |h, ctx, ev| h.on_join(ctx, &ev.player().clone())));
        file(adapt::<QuitEvent>(priority, &shared, |h, ctx, ev| h.on_quit(ctx, &ev.player().clone())));
        file(adapt::<WorldChangeEvent>(priority, &shared, |h, ctx, ev| h.on_world_change(ctx, &ev.player().clone(), ev.from(), ev.to())));
        file(adapt::<TransferFailedEvent>(priority, &shared, |h, ctx, ev| h.on_transfer_failed(ctx, &ev.player().clone(), ev.to())));
        file(adapt::<ChatEvent>(priority, &shared, |h, ctx, ev| h.on_chat(ctx, &ev.player().clone(), ev)));
        file(adapt::<CommandEvent>(priority, &shared, |h, ctx, ev| h.on_command(ctx, &ev.player().clone(), ev)));
        file(adapt::<BreakEvent>(priority, &shared, |h, ctx, ev| h.on_break(ctx, &ev.player().clone(), ev)));
        file(adapt::<PlaceEvent>(priority, &shared, |h, ctx, ev| h.on_place(ctx, &ev.player().clone(), ev)));
        file(adapt::<InteractEvent>(priority, &shared, |h, ctx, ev| h.on_interact(ctx, &ev.player().clone(), ev)));
        file(adapt::<DropEvent>(priority, &shared, |h, ctx, ev| h.on_drop(ctx, &ev.player().clone(), ev)));
        file(adapt::<TickEvent>(priority, &shared, |h, ctx, _| h.on_tick(ctx)));
        self
    }

    fn scope(&self) -> Scope {
        Scope::Shard
    }

    fn submit(&mut self, change: ListenerChange) {
        self.listeners.apply(change);
    }
}

type Shared = Arc<Mutex<Box<dyn Handler>>>;

fn adapt<E: Event>(priority: Priority, handler: &Shared, call: fn(&mut dyn Handler, &mut ShardCtx<'_>, &mut E)) -> ListenerChange {
    let handler = handler.clone();
    ListenerChange::listen(Scope::Shard, priority.into(), move |ctx, event: &mut E| call(handler.lock().expect("handler lock").as_mut(), ctx, event)).1
}

/// Listeners and tasks a running listener adds in one scope ([`ShardCtx::in_world`]); they take
/// part from the next event on.
pub struct Scoped<'c, 'a> {
    ctx: &'c mut ShardCtx<'a>,
    scope: Scope,
}

impl Scoped<'_, '_> {
    fn scope(&self) -> Scope {
        self.scope.clone()
    }

    fn submit(&mut self, change: ListenerChange) {
        self.ctx.backend.defer(change);
    }
}

macro_rules! registrar {
    ($($ty:ty),*) => {$(
        impl $ty {
            /// Calls `call` for every `E` in this scope (see [`Listen`] and [`Priority`]).
            pub fn on<E: Event>(&mut self, listen: impl Into<Listen>, call: impl FnMut(&mut ShardCtx<'_>, &mut E) + Send + 'static) -> ListenerId {
                let (id, change) = ListenerChange::listen(self.scope(), listen.into(), call);
                self.submit(change);
                id
            }

            /// Runs `run` every `ticks` ticks, the first time `ticks` from now.
            pub fn every(&mut self, ticks: u64, run: impl FnMut(&mut ShardCtx<'_>) + Send + 'static) -> ListenerId {
                self.schedule(ticks, Some(ticks), Box::new(run))
            }

            /// Runs `run` once, `ticks` from now.
            pub fn later(&mut self, ticks: u64, run: impl FnOnce(&mut ShardCtx<'_>) + Send + 'static) -> ListenerId {
                let mut run = Some(run);
                self.schedule(ticks, None, Box::new(move |ctx| {
                    if let Some(run) = run.take() {
                        run(ctx);
                    }
                }))
            }

            fn schedule(&mut self, delay: u64, period: Option<u64>, run: Run) -> ListenerId {
                let (id, change) = ListenerChange::task(self.scope(), delay, period, run);
                self.submit(change);
                id
            }
        }
    )*};
}
registrar!(PluginBuilder<'_>, Scoped<'_, '_>);

impl<'a> ShardCtx<'a> {
    /// Adds listeners and tasks for the whole shard.
    pub fn on_shard(&mut self) -> Scoped<'_, 'a> {
        Scoped { ctx: self, scope: Scope::Shard }
    }

    /// Adds listeners and tasks for one world of this shard, dropped when it is removed. `None`
    /// for a world on another shard.
    pub fn in_world(&mut self, world: &str) -> Option<Scoped<'_, 'a>> {
        self.within(world, |_| ())?;
        Some(Scoped { ctx: self, scope: Scope::World(world.to_owned()) })
    }

    /// Adds listeners and tasks for one player in a world of this shard, dropped when they leave
    /// it. `None` for anyone else.
    pub fn for_player(&mut self, player: PlayerId) -> Option<Scoped<'_, 'a>> {
        self.world_of(player)?;
        Some(Scoped { ctx: self, scope: Scope::Player(player) })
    }

    /// Removes a listener or task, from the next event on.
    pub fn remove_listener(&mut self, id: ListenerId) {
        self.backend.defer(ListenerChange::remove(id));
    }
}
