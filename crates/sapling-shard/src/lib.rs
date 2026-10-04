//! A shard: one thread, one `bevy_ecs` world, ticking at 20 Hz. It drains the network inbox at the
//! start of each tick, runs its systems in a fixed order, calls plugin handlers, tells players
//! about each other and hands each player's packets to the outbox as one batch. See
//! docs/DESIGN.md "Threads", "Worlds" and "Step 3 as built".

mod blobs;
mod chat;
mod chunks;
mod components;
mod handlers;
mod join;
mod movement;
mod roster;
mod shard;
mod skin;
mod view;
mod visibility;

pub use shard::{Shard, ShardConfig, ShardHandle, TICK};
