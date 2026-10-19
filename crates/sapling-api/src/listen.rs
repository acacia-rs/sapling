//! Typed listeners and timed tasks, each in a scope, and the registry a shard runs them from
//! (docs/DESIGN.md "Plugin API").

use std::any::{Any, TypeId};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::{Player, PlayerId, Priority, ShardCtx};

/// Something listeners can be registered for ([`crate::PluginBuilder::on`]).
pub trait Event: Clone + Send + 'static {
    /// The player it is about: a listener in [`Scope::Player`] sees only that player's.
    fn player(&self) -> Option<&Player> {
        None
    }

    /// Whether a listener cancelled it, which skips the listeners after it (see [`Priority`]).
    fn is_stopped(&self) -> bool {
        false
    }
}

/// Where a listener or task applies. One in a world or for a player is dropped once the world is
/// removed or the player leaves the shard.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Scope {
    /// Every world of the shard.
    Shard,
    /// Events while that world is the one in focus: its players', its blocks'. Its tasks run
    /// with it in focus.
    World(String),
    Player(PlayerId),
}

/// How a listener runs: its priority, and whether it still sees an event that a listener before
/// it cancelled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Listen {
    pub priority: Priority,
    pub receive_cancelled: bool,
}

impl From<Priority> for Listen {
    fn from(priority: Priority) -> Self {
        Self { priority, receive_cancelled: false }
    }
}

impl Priority {
    /// This priority, for a listener that runs even after another cancelled the event.
    pub fn receiving_cancelled(self) -> Listen {
        Listen { priority: self, receive_cancelled: true }
    }
}

/// A listener or task, for [`ShardCtx::remove_listener`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ListenerId(u64);

impl ListenerId {
    fn next() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

type Call<E> = Box<dyn FnMut(&mut ShardCtx<'_>, &mut E) + Send>;
pub(crate) type Run = Box<dyn FnMut(&mut ShardCtx<'_>) + Send>;

struct Listener {
    id: ListenerId,
    scope: Scope,
    listen: Listen,
    event: TypeId,
    /// A `Call<E>` for the `E` of `event`.
    call: Box<dyn Any + Send>,
}

struct Task {
    id: ListenerId,
    scope: Scope,
    delay: u64,
    period: Option<u64>,
    /// Set by the first [`Listeners::run_tasks`] after the task was added.
    due: Option<u64>,
    run: Run,
}

/// A change to a shard's [`Listeners`]. One made while an event runs waits for it to finish
/// ([`crate::Backend::defer`]).
pub struct ListenerChange(Change);

enum Change {
    Listen(Listener),
    Task(Task),
    Remove(ListenerId),
    End(Scope),
}

impl ListenerChange {
    /// Drops everything in `scope`: the engine's word that a world or a player is gone.
    pub fn end(scope: Scope) -> Self {
        Self(Change::End(scope))
    }

    pub(crate) fn remove(id: ListenerId) -> Self {
        Self(Change::Remove(id))
    }

    pub(crate) fn listen<E: Event>(scope: Scope, listen: Listen, call: impl FnMut(&mut ShardCtx<'_>, &mut E) + Send + 'static) -> (ListenerId, Self) {
        let (id, call): (_, Call<E>) = (ListenerId::next(), Box::new(call));
        (id, Self(Change::Listen(Listener { id, scope, listen, event: TypeId::of::<E>(), call: Box::new(call) })))
    }

    pub(crate) fn task(scope: Scope, delay: u64, period: Option<u64>, run: Run) -> (ListenerId, Self) {
        let id = ListenerId::next();
        (id, Self(Change::Task(Task { id, scope, delay, period, due: None, run })))
    }
}

/// A shard's listeners, in priority order, and its tasks. The engine runs events and ticks
/// through it; plugins reach it through [`crate::PluginBuilder`] and [`ShardCtx::in_world`].
#[derive(Default)]
pub struct Listeners {
    listeners: Vec<Listener>,
    tasks: Vec<Task>,
}

impl Listeners {
    pub fn apply(&mut self, change: ListenerChange) {
        match change.0 {
            Change::Listen(listener) => {
                // Equal priorities keep the order they were added in.
                let at = self.listeners.partition_point(|l| l.listen.priority <= listener.listen.priority);
                self.listeners.insert(at, listener);
            }
            Change::Task(task) => self.tasks.push(task),
            Change::Remove(id) => {
                self.listeners.retain(|l| l.id != id);
                self.tasks.retain(|t| t.id != id);
            }
            Change::End(scope) => {
                self.listeners.retain(|l| l.scope != scope);
                self.tasks.retain(|t| t.scope != scope);
            }
        }
    }

    /// Runs `event` through the listeners in its scope, in priority order, and returns how it
    /// ended. Once one cancels it, only `Monitor` listeners and those receiving cancelled events
    /// still run; `Monitor` ones get a copy of it as it ended, whatever they do to it.
    pub fn offer<E: Event>(&mut self, ctx: &mut ShardCtx<'_>, mut event: E) -> E {
        let mut world = None;
        for listener in self.listeners.iter_mut().filter(|l| l.event == TypeId::of::<E>()) {
            let applies = match &listener.scope {
                Scope::Shard => true,
                Scope::World(name) => world.get_or_insert_with(|| ctx.world()) == name,
                Scope::Player(id) => event.player().is_some_and(|p| p.id == *id),
            };
            let call = listener.call.downcast_mut::<Call<E>>().expect("a listener is filed under its event's type");
            if !applies {
                continue;
            } else if listener.listen.priority == Priority::Monitor {
                call(ctx, &mut event.clone());
            } else if listener.listen.receive_cancelled || !event.is_stopped() {
                call(ctx, &mut event);
            }
        }
        event
    }

    /// Runs the tasks due at `ctx.tick()`. A world's runs with the world in focus, and is dropped
    /// once the world is not on the shard any more.
    pub fn run_tasks(&mut self, ctx: &mut ShardCtx<'_>) {
        let now = ctx.tick();
        self.tasks.retain_mut(|task| {
            if *task.due.get_or_insert(now + task.delay) > now {
                return true;
            }
            let ran = match &task.scope {
                Scope::World(name) => ctx.within(name, |ctx| (task.run)(ctx)).is_some(),
                Scope::Shard | Scope::Player(_) => {
                    (task.run)(ctx);
                    true
                }
            };
            task.due = task.period.map(|period| now + period.max(1));
            ran && task.due.is_some()
        });
    }
}
