//! A shard: one thread, one `bevy_ecs` world, ticking at 20 Hz. It drains the network inbox at the
//! start of each tick, runs its systems in a fixed order, calls plugin handlers and hands each
//! player's packets to the outbox as one batch. See docs/DESIGN.md "Threads" and "Worlds".

mod blobs;
mod chunks;
mod components;
mod handlers;
mod join;
mod shard;
mod view;

pub use shard::{Shard, ShardConfig, ShardHandle, TICK};
