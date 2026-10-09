//! What a server adds to a shard: handlers in priority order and the command registry.

use std::any::TypeId;
use std::collections::HashMap;

use bevy_ecs::prelude::*;
use sapling_api::{Cancellable, Commands, Handler, Priority, ShardCtx, State};

use crate::handlers::Ctx;

/// What plugins keep on the shard, by type (`ShardCtx::state`).
#[derive(Resource, Default)]
pub(crate) struct PluginState(pub HashMap<TypeId, State>);

#[derive(Default)]
pub(crate) struct Plugins {
    /// Sorted by priority (`Monitor` last); equal priorities in the order they were added.
    handlers: Vec<(Priority, Box<dyn Handler>)>,
    pub commands: Commands,
}

impl Plugins {
    pub fn add(&mut self, priority: Priority, handler: Box<dyn Handler>) {
        let at = self.handlers.partition_point(|(p, _)| *p <= priority);
        self.handlers.insert(at, (priority, handler));
    }

    /// Calls every handler for an event that cannot be stopped.
    pub fn each(
        &mut self,
        world: &mut World,
        mut call: impl FnMut(&mut dyn Handler, &mut ShardCtx<'_>),
    ) {
        for (_, handler) in &mut self.handlers {
            let mut backend = Ctx { world };
            call(handler.as_mut(), &mut ShardCtx::new(&mut backend));
        }
    }

    /// Runs `event` through the handlers until one cancels it and returns how it ended. `Monitor`
    /// handlers get a copy of that end state, cancelled or not (see [`Priority`]).
    pub fn offer<E: Cancellable>(
        &mut self,
        world: &mut World,
        mut event: E,
        mut call: impl FnMut(&mut dyn Handler, &mut ShardCtx<'_>, &mut E),
    ) -> E {
        for (priority, handler) in &mut self.handlers {
            let mut backend = Ctx { world };
            let ctx = &mut ShardCtx::new(&mut backend);
            if *priority == Priority::Monitor {
                call(handler.as_mut(), ctx, &mut event.clone());
            } else if !event.is_stopped() {
                call(handler.as_mut(), ctx, &mut event);
            }
        }
        event
    }
}
