//! Packs on the builder: resource packs for clients, behavior packs' functions for the server.

use std::sync::Arc;

use crate::ServerBuilder;
use crate::functions::FunctionsPlugin;
use crate::packs::{BehaviorPacks, ResourcePacks};

impl ServerBuilder {
    /// Has joining clients download `packs` (none by default).
    pub fn resource_packs(mut self, packs: ResourcePacks) -> Self {
        self.shard.resource_packs = Arc::new(packs.build());
        self
    }

    /// Runs the functions of `packs`: adds `/function` and `/schedule` for operators, and runs
    /// what each `tick.json` lists every tick, in every world.
    pub fn behavior_packs(self, packs: BehaviorPacks) -> Self {
        self.plugin(FunctionsPlugin(Arc::new(packs.build())))
    }
}
