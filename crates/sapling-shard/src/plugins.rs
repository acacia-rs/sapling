//! What a server adds to a shard: its listeners and tasks, and its commands. Both live in the
//! world, so any of the game's code can raise an event ([`offer`]).

use std::any::TypeId;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use bevy_ecs::prelude::*;
use sapling_api::{Commands, Event, ListenerChange, Listeners, PluginBuilder, Scope, ShardCtx, State};

use crate::handlers::Ctx;

/// What plugins keep on the shard, by type (`ShardCtx::state`).
#[derive(Resource, Default)]
pub(crate) struct PluginState(pub HashMap<TypeId, State>);

/// The shard's listeners and tasks, taken out while they run: what a listener's own doing raises
/// is not offered to them again, so a listener cannot set itself off. Locked because a resource
/// must be `Sync` and a listener is only `Send`.
#[derive(Resource)]
pub(crate) struct Registry(Mutex<Option<Listeners>>);

impl Default for Registry {
    fn default() -> Self {
        Self(Mutex::new(Some(Listeners::default())))
    }
}

/// The shard's commands, shared so that one runs without holding the world.
#[derive(Resource, Default)]
pub(crate) struct CommandRegistry(pub Arc<Commands>);

/// Changes to the listeners made while they ran, applied once the registry is back.
#[derive(Resource, Default)]
pub(crate) struct Deferred(Mutex<Vec<ListenerChange>>);

impl Deferred {
    pub fn push(&self, change: ListenerChange) {
        self.0.lock().expect("deferred listeners lock").push(change);
    }
}

/// Drops the listeners and tasks of a world or a player that is gone from the shard.
pub(crate) fn end_scope(world: &World, scope: Scope) {
    world.resource::<Deferred>().push(ListenerChange::end(scope));
}

/// Adds listeners, tasks and commands between ticks; the caller tells players of new commands.
pub(crate) fn build(world: &mut World, add: impl FnOnce(&mut PluginBuilder<'_>)) {
    with_listeners(world, |world, listeners| {
        let mut registry = world.resource_mut::<CommandRegistry>();
        let commands = Arc::get_mut(&mut registry.0).expect("no command runs while plugins are added");
        add(&mut PluginBuilder::new(listeners, commands));
    });
}

/// Runs `event` through the listeners and returns how it ended ([`Listeners::offer`]). Raised by
/// a running listener's own doing, it goes by unheard.
pub(crate) fn offer<E: Event>(world: &mut World, event: E) -> E {
    let mut event = Some(event);
    with_listeners(world, |world, listeners| event = event.take().map(|event| listeners.offer(&mut ShardCtx::new(&mut Ctx { world }), event)));
    event.expect("offered or passed by")
}

pub(crate) fn run_tasks(world: &mut World) {
    with_listeners(world, |world, listeners| listeners.run_tasks(&mut ShardCtx::new(&mut Ctx { world })));
}

/// Runs `f` with the registry taken out, the changes made meanwhile applied before and after;
/// does nothing while the registry is out.
fn with_listeners(world: &mut World, f: impl FnOnce(&mut World, &mut Listeners)) {
    let Some(mut listeners) = world.resource::<Registry>().0.lock().expect("listener registry lock").take() else { return };
    apply_deferred(world, &mut listeners);
    f(world, &mut listeners);
    apply_deferred(world, &mut listeners);
    *world.resource::<Registry>().0.lock().expect("listener registry lock") = Some(listeners);
}

fn apply_deferred(world: &World, listeners: &mut Listeners) {
    let changes = std::mem::take(&mut *world.resource::<Deferred>().0.lock().expect("deferred listeners lock"));
    changes.into_iter().for_each(|change| listeners.apply(change));
}
