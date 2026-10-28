//! Packs on the builder: resource packs for clients, behavior packs' functions for the server.

use std::sync::Arc;

use crate::ServerBuilder;
use crate::functions::{FunctionsPlugin, Library};
use crate::packs::{BehaviorPacks, ResourcePacks};

impl ServerBuilder {
    /// Has joining clients download `packs` (none by default).
    pub fn resource_packs(mut self, packs: ResourcePacks) -> Self {
        self.shard.resource_packs = Arc::new(packs.build());
        self
    }

    /// Runs the functions of `packs`: adds `/function`, `/schedule` and `/reload` (which reads
    /// the packs again) for operators, and runs what each `tick.json` lists every tick, in every
    /// world.
    pub fn behavior_packs(self, packs: BehaviorPacks) -> Self {
        self.plugin(FunctionsPlugin(Library::new(packs)))
    }
}
