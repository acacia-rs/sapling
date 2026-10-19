//! What a server adds to a shard: its listeners and tasks, and the command registry.

use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Mutex;

use bevy_ecs::prelude::*;
use sapling_api::{Commands, Event, Handler, ListenerChange, Listeners, Plugin, PluginBuilder, Priority, Scope, ShardCtx, State};

use crate::handlers::Ctx;

/// What plugins keep on the shard, by type (`ShardCtx::state`).
#[derive(Resource, Default)]
pub(crate) struct PluginState(pub HashMap<TypeId, State>);

/// Changes to the listeners made while they ran, applied once the registry is free. Locked
/// because a resource must be `Sync` and a listener is only `Send`.
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

#[derive(Default)]
pub(crate) struct Plugins {
    listeners: Listeners,
    pub commands: Commands,
}

impl Plugins {
    pub fn add_handler(&mut self, priority: Priority, handler: Box<dyn Handler>) {
        PluginBuilder::new(&mut self.listeners, &mut self.commands).handler(priority, handler);
    }

    pub fn add_plugin(&mut self, plugin: &dyn Plugin) {
        plugin.build(&mut PluginBuilder::new(&mut self.listeners, &mut self.commands));
    }

    /// Runs `event` through the listeners and returns how it ended ([`Listeners::offer`]).
    pub fn offer<E: Event>(&mut self, world: &mut World, event: E) -> E {
        self.apply_deferred(world);
        let event = self.listeners.offer(&mut ShardCtx::new(&mut Ctx { world }), event);
        self.apply_deferred(world);
        event
    }

    pub fn run_tasks(&mut self, world: &mut World) {
        self.apply_deferred(world);
        self.listeners.run_tasks(&mut ShardCtx::new(&mut Ctx { world }));
        self.apply_deferred(world);
    }

    fn apply_deferred(&mut self, world: &World) {
        let changes = std::mem::take(&mut *world.resource::<Deferred>().0.lock().expect("deferred listeners lock"));
        changes.into_iter().for_each(|change| self.listeners.apply(change));
    }
}
